use std::path::PathBuf;

use crate::{
    net::{image_filename, ImageKind, ImageSize},
    utils::{
        event_hash_or_err, millis_to_naive_or_err, profile_meta_or_err, public_key_or_err,
        url_or_err,
    },
};
use chrono::NaiveDateTime;
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;

use super::DbEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing JSON content into nostr::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("Not found path for kind: {0:?}")]
    NoPathForKind(ImageKind),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCache {
    pub public_key: XOnlyPublicKey,
    pub updated_at: NaiveDateTime,
    pub event_hash: nostr::EventId,
    pub from_relay: nostr::Url,
    pub metadata: nostr::Metadata,
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

    pub async fn insert_with_event(
        cache_pool: &SqlitePool,
        db_event: &DbEvent,
    ) -> Result<u64, Error> {
        let metadata = nostr::Metadata::from_json(&db_event.content)
            .map_err(|_| Error::JsonToMetadata(db_event.content.clone()))?;
        let public_key = &db_event.pubkey;
        let event_hash = &db_event.event_hash;
        let event_date = &db_event
            .remote_creation()
            .ok_or(Error::NotConfirmedEvent(event_hash.to_owned()))?;
        let relay_url = db_event
            .relay_url
            .as_ref()
            .ok_or(Error::NotConfirmedEvent(event_hash.to_owned()))?;
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
                    "Skipping update. Outdated event for pubkey: {} - cache: {:?} - event: {:?}",
                    public_key.to_string(),
                    last_cache.updated_at,
                    event_date
                );
                return Ok(0);
            }
        }

        let mut tx = cache_pool.begin().await?;

        let update_query = r#"
            UPDATE profile_meta_cache 
            SET updated_at=?, event_hash=?, metadata=?, from_relay=?
            WHERE public_key = ?
        "#;
        let mut rows_affected = sqlx::query(&update_query)
            .bind(&event_date.timestamp_millis())
            .bind(&event_hash.to_string())
            .bind(&metadata.as_json())
            .bind(&relay_url.to_string())
            .bind(&public_key.to_string())
            .execute(&mut tx)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            let insert_query = r#"
                INSERT INTO profile_meta_cache
                    (public_key, updated_at, event_hash, metadata, from_relay) 
                VALUES (?, ?, ?, ?, ?)
            "#;
            rows_affected = sqlx::query(&insert_query)
                .bind(&public_key.to_string())
                .bind(&event_date.timestamp_millis())
                .bind(&event_hash.to_string())
                .bind(&metadata.as_json())
                .bind(&relay_url.to_string())
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

    pub(crate) async fn remove_file(
        cache_pool: &SqlitePool,
        cache: &ProfileCache,
        kind: ImageKind,
    ) -> Result<(), Error> {
        let path = cache.get_path(kind).ok_or(Error::NoPathForKind(kind))?;
        if path.exists() {
            remove_all_images(&path, kind).await?;
        }
        let kind_str = match kind {
            ImageKind::Profile => "profile_image_path",
            ImageKind::Banner => "banner_image_path",
        };
        let update_query = format!(
            r#"
            UPDATE profile_meta_cache 
            SET {}=?
            WHERE public_key=?
        "#,
            kind_str
        );
        sqlx::query(&update_query)
            .bind(&"".to_string())
            .bind(&cache.public_key.to_string())
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

        let from_relay = row.try_get::<String, &str>("from_relay")?;
        let from_relay = url_or_err(&from_relay, "from_relay")?;

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
            from_relay,
            profile_image_path,
            banner_image_path,
        })
    }
}

async fn remove_all_images(path: &PathBuf, kind: ImageKind) -> Result<(), Error> {
    tokio::fs::remove_file(path).await?;

    let med_path = image_filename(kind, ImageSize::Medium, "png");
    tokio::fs::remove_file(med_path).await?;

    let sm_path = image_filename(kind, ImageSize::Small, "png");
    tokio::fs::remove_file(sm_path).await?;
    Ok(())
}
