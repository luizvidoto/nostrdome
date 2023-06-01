use thiserror::Error;

use chrono::NaiveDateTime;
use nostr::secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    types::ChannelMetadata,
    utils::{channel_meta_or_err, event_hash_or_err, millis_to_naive_or_err, public_key_or_err},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Error parsing JSON content into nostr::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Can't update channel without id")]
    ChannelNotInDatabase,

    #[error("Not found channel to update: channel_id: {0}")]
    NotFoundChannelToUpdate(nostr::EventId),

    #[error("Not found channel id inside event tags: event_hash: {0}")]
    ChannelIdNotFound(nostr::EventId),

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(nostr::EventId),
}

use super::DbEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCache {
    pub channel_id: nostr::EventId,
    pub creator_pubkey: XOnlyPublicKey,
    pub created_at: NaiveDateTime,
    pub updated_event_hash: Option<nostr::EventId>,
    pub updated_at: Option<NaiveDateTime>,
    pub metadata: ChannelMetadata,
}
impl ChannelCache {
    pub async fn fetch_by_creator(
        cache_pool: &SqlitePool,
        creator_pubkey: &XOnlyPublicKey,
    ) -> Result<Vec<ChannelCache>, Error> {
        let query = "SELECT * FROM channel_cache WHERE creator_pubkey = ?;";
        let result = sqlx::query_as::<_, ChannelCache>(query)
            .bind(creator_pubkey.to_string())
            .fetch_all(cache_pool)
            .await?;
        Ok(result)
    }

    pub async fn fetch_by_channel_id(
        cache_pool: &SqlitePool,
        channel_id: &nostr::EventId,
    ) -> Result<Option<ChannelCache>, Error> {
        let query = "SELECT * FROM channel_cache WHERE creation_event_hash = ?;";
        let result = sqlx::query_as::<_, ChannelCache>(query)
            .bind(channel_id.to_string())
            .fetch_optional(cache_pool)
            .await?;
        Ok(result)
    }

    pub async fn insert(
        cache_pool: &SqlitePool,
        db_event: &DbEvent,
    ) -> Result<ChannelCache, Error> {
        let metadata = nostr::Metadata::from_json(&db_event.content)
            .map_err(|_| Error::JsonToMetadata(db_event.content.clone()))?;
        let channel_id = &db_event.event_hash;
        let creator_pubkey = &db_event.pubkey;
        let created_at = &db_event
            .remote_creation()
            .ok_or(Error::NotConfirmedEvent(channel_id.to_owned()))?;

        if let Some(channel_cache) = Self::fetch_by_channel_id(cache_pool, channel_id).await? {
            return Ok(channel_cache);
        }

        let insert_query = r#"
            INSERT INTO channel_cache
                (creation_event_hash, creator_pubkey, created_at, metadata)
            VALUES (?1, ?2, ?3, ?4)
        "#;
        sqlx::query(&insert_query)
            .bind(channel_id.to_string())
            .bind(creator_pubkey.to_string())
            .bind(created_at.timestamp_millis())
            .bind(metadata.as_json())
            .execute(cache_pool)
            .await?;

        let channel_cache = Self::fetch_by_channel_id(cache_pool, channel_id)
            .await?
            .ok_or(Error::ChannelNotInDatabase)?;

        Ok(channel_cache)
    }
    pub async fn update(
        cache_pool: &SqlitePool,
        db_event: &DbEvent,
    ) -> Result<ChannelCache, Error> {
        // Only updates for kind 41 coming from the same channel_id.
        // the channel id is inside the E tag
        // It's possible to receive a kind 41 before a kind 40.
        let channel_id = channel_id_from_tags(&db_event.tags)
            .ok_or(Error::ChannelIdNotFound(db_event.event_hash.to_owned()))?;

        let channel_cache = Self::fetch_by_channel_id(cache_pool, &channel_id)
            .await?
            .ok_or(Error::NotFoundChannelToUpdate(channel_id.to_owned()))?;

        let metadata = nostr::Metadata::from_json(&db_event.content)
            .map_err(|_| Error::JsonToMetadata(db_event.content.clone()))?;
        let updated_event_hash = db_event.event_hash;
        let updated_at = db_event
            .remote_creation()
            .ok_or(Error::NotConfirmedEvent(updated_event_hash.clone()))?;

        let update_query = r#"
            UPDATE channel_cache
            SET metadata=?, updated_event_hash=?, updated_at=?
            WHERE channel_id = ?
        "#;

        sqlx::query(&update_query)
            .bind(metadata.as_json())
            .bind(updated_event_hash.to_string())
            .bind(updated_at.timestamp_millis())
            .bind(channel_id.to_string())
            .execute(cache_pool)
            .await?;

        let channel_cache = Self::fetch_by_channel_id(cache_pool, &channel_id)
            .await?
            .ok_or(Error::ChannelNotInDatabase)?;

        Ok(channel_cache)
    }
}

impl sqlx::FromRow<'_, SqliteRow> for ChannelCache {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let metadata: String = row.try_get("metadata")?;
        let metadata = channel_meta_or_err(&metadata, "metadata")?;

        let channel_id: String = row.try_get("creation_event_hash")?;
        let channel_id = event_hash_or_err(&channel_id, "creation_event_hash")?;

        let updated_event_hash: Option<String> = row.get("updated_event_hash");
        let updated_event_hash = updated_event_hash
            .map(|h| event_hash_or_err(&h, "updated_event_hash"))
            .transpose()?;

        let creator_pubkey = row.try_get::<String, &str>("creator_pubkey")?;
        let creator_pubkey = public_key_or_err(&creator_pubkey, "creator_pubkey")?;

        let created_at: i64 = row.try_get("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        let updated_at: Option<i64> = row.get("updated_at");
        let updated_at = updated_at
            .map(|date| millis_to_naive_or_err(date, "updated_at"))
            .transpose()?;

        Ok(Self {
            metadata,
            created_at,
            updated_at,
            channel_id,
            creator_pubkey,
            updated_event_hash,
        })
    }
}

fn channel_id_from_tags(tags: &[nostr::Tag]) -> Option<nostr::EventId> {
    tags.iter().find_map(|tag| {
        if let nostr::Tag::Event(event_id, _, _) = tag {
            Some(event_id.to_owned())
        } else {
            None
        }
    })
}
