use crate::{
    net::ImageKind,
    utils::{
        event_hash_or_err, millis_to_naive_or_err, ns_event_to_naive, profile_meta_or_err,
        public_key_or_err, url_or_err,
    },
};
use chrono::NaiveDateTime;
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;
use url::Url;

use super::ImageDownloaded;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing JSON content into nostr::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    FromImageCache(#[from] crate::db::image_cache::Error),

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(nostr::Timestamp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCache {
    pub public_key: XOnlyPublicKey,
    pub updated_at: NaiveDateTime,
    pub event_hash: nostr::EventId,
    pub from_relay: nostr::Url,
    pub metadata: nostr::Metadata,
    pub profile_pic_cache: Option<ImageDownloaded>,
    pub banner_pic_cache: Option<ImageDownloaded>,
}
impl ProfileCache {
    pub async fn fetch_by_public_key(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
    ) -> Result<Option<ProfileCache>, Error> {
        let query = "SELECT * FROM profile_meta_cache WHERE public_key = ?;";
        let mut result = sqlx::query_as::<_, ProfileCache>(query)
            .bind(&public_key.to_string())
            .fetch_optional(cache_pool)
            .await?;

        if let Some(profile_cache) = &mut result {
            profile_cache.profile_pic_cache =
                ImageDownloaded::fetch(cache_pool, &profile_cache.event_hash, ImageKind::Profile)
                    .await?;
            profile_cache.banner_pic_cache =
                ImageDownloaded::fetch(cache_pool, &profile_cache.event_hash, ImageKind::Banner)
                    .await?;
        }

        Ok(result)
    }

    // pub async fn fetch_channel_members(
    //     cache_pool: &SqlitePool,
    //     channel_id: &EventId,
    // ) -> Result<Vec<Self>, Error> {
    //     let sql = r#"
    //         SELECT profile_meta_cache.*
    //         FROM channel_member_map
    //         INNER JOIN profile_meta_cache ON channel_member_map.public_key = profile_meta_cache.public_key
    //         WHERE channel_member_map.channel_id = ?;
    //     "#;
    //     let members = sqlx::query_as::<_, Self>(sql)
    //         .bind(channel_id.to_string())
    //         .fetch_all(cache_pool)
    //         .await?;

    //     Ok(members)
    // }

    pub async fn insert(
        cache_pool: &SqlitePool,
        relay_url: &Url,
        ns_event: nostr::Event,
    ) -> Result<u64, Error> {
        let metadata = nostr::Metadata::from_json(&ns_event.content)
            .map_err(|_| Error::JsonToMetadata(ns_event.content.clone()))?;
        let public_key = &ns_event.pubkey;
        let event_hash = &ns_event.id;
        let event_date = ns_event_to_naive(ns_event.created_at)
            .map_err(|_| Error::InvalidTimestamp(ns_event.created_at))?;

        if let Some(last_cache) = Self::fetch_by_public_key(cache_pool, public_key).await? {
            if &last_cache.event_hash == event_hash {
                tracing::debug!(
                    "Skipping update. Same event id for pubkey: {}",
                    public_key.to_string()
                );
                return Ok(0);
            }
            if last_cache.updated_at > event_date {
                tracing::debug!(
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
        let mut rows_affected = sqlx::query(update_query)
            .bind(event_date.timestamp_millis())
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
            rows_affected = sqlx::query(insert_query)
                .bind(&public_key.to_string())
                .bind(event_date.timestamp_millis())
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

        Ok(Self {
            public_key,
            updated_at,
            event_hash,
            metadata,
            from_relay,
            profile_pic_cache: None,
            banner_pic_cache: None,
        })
    }
}
