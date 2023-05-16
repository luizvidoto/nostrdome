use std::path::PathBuf;

use chrono::NaiveDateTime;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::Error,
    net::ImageKind,
    utils::{event_hash_or_err, millis_to_naive_or_err, profile_meta_or_err, public_key_or_err},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCache {
    pub public_key: XOnlyPublicKey,
    pub updated_at: NaiveDateTime,
    pub event_hash: nostr_sdk::EventId,
    pub metadata: nostr_sdk::Metadata,
    pub profile_image_path: Option<PathBuf>,
    pub banner_image_path: Option<PathBuf>,
}
impl ProfileCache {
    pub async fn fetch_by_public_key(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
    ) -> Result<Option<ProfileCache>, Error> {
        let query = "SELECT * FROM profile_meta_cache WHERE public_key = ?;";
        let result = sqlx::query_as::<_, ProfileCache>(query)
            .bind(&public_key.to_string())
            .fetch_optional(cache_pool)
            .await?;
        Ok(result)
    }

    // if let Some(profile_cache) = ProfileCache::fetch_by_public_key(cache_pool, public_key).await? {
    //     if event_date < &profile_cache.updated_at {
    //         if let Some(path_of_kind) = profile_cache.get_path(kind) {
    //             return Ok(path_of_kind);
    //         }
    //     }
    //     if let Some(url_of_kind) = profile_cache.get_url(kind) {
    //         if image_url == url_of_kind {
    //             if let Some(path_of_kind) = profile_cache.get_path(kind) {
    //                 return Ok(path_of_kind);
    //             }
    //         }
    //     }
    // }

    pub async fn insert_by_public_key(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
        event_hash: &nostr_sdk::EventId,
        event_date: &NaiveDateTime,
        metadata: &nostr_sdk::Metadata,
    ) -> Result<u64, Error> {
        if let Some(last_cache) = Self::fetch_by_public_key(cache_pool, public_key).await? {
            if &last_cache.event_hash == event_hash {
                tracing::info!(
                    "Skipping update. Same event id for pubkey: {}",
                    public_key.to_string()
                );
                return Ok(0);
            }
            if last_cache.updated_at > *event_date {
                tracing::warn!(
                    "Skipping updated. Outdated event for pubkey: {}",
                    public_key.to_string()
                );
                return Ok(0);
            }
        }

        let mut tx = cache_pool.begin().await?;

        let update_query = r#"
            UPDATE profile_meta_cache 
            SET updated_at=?, event_hash=?, metadata=?
            WHERE public_key = ?
        "#;
        let mut rows_affected = sqlx::query(&update_query)
            .bind(&event_date.timestamp_millis())
            .bind(&event_hash.to_string())
            .bind(&metadata.as_json())
            .bind(&public_key.to_string())
            .execute(&mut tx)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            let insert_query = r#"
                INSERT INTO profile_meta_cache
                    (public_key, updated_at, event_hash, metadata) 
                VALUES (?, ?, ?, ?)
            "#;
            rows_affected = sqlx::query(&insert_query)
                .bind(&public_key.to_string())
                .bind(&event_date.timestamp_millis())
                .bind(&event_hash.to_string())
                .bind(&metadata.as_json())
                .execute(&mut tx)
                .await?
                .rows_affected();
        }

        tx.commit().await?;

        Ok(rows_affected)
    }

    pub(crate) async fn update_local_path(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
        kind: ImageKind,
        path: &PathBuf,
    ) -> Result<(), Error> {
        let kind_str = match kind {
            ImageKind::Profile => "profile_image_path",
            ImageKind::Banner => "banner_image_path",
        };
        let update_query = format!(
            r#"
            UPDATE profile_meta_cache 
            SET {}=?
            WHERE public_key = ?
        "#,
            kind_str
        );
        sqlx::query(&update_query)
            .bind(&path.to_string_lossy())
            .bind(&public_key.to_string())
            .execute(cache_pool)
            .await?;
        Ok(())
    }

    pub(crate) fn get_path(&self, kind: ImageKind) -> Option<PathBuf> {
        match kind {
            ImageKind::Profile => self.profile_image_path.clone(),
            ImageKind::Banner => self.banner_image_path.clone(),
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for ProfileCache {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let metadata: String = row.try_get("metadata")?;
        let metadata = profile_meta_or_err(&metadata, "metadata")?;

        let event_hash: String = row.try_get("event_hash")?;
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;

        let public_key = row.try_get::<String, &str>("public_key")?;
        let public_key = public_key_or_err(&public_key, "public_key")?;

        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;

        let profile_image_path: Option<String> = row.get("profile_image_path");
        let profile_image_path = profile_image_path.map(|path| PathBuf::from(path));

        let banner_image_path: Option<String> = row.get("banner_image_path");
        let banner_image_path = banner_image_path.map(|path| PathBuf::from(path));

        Ok(Self {
            public_key,
            updated_at,
            event_hash,
            metadata,
            profile_image_path,
            banner_image_path,
        })
    }
}
