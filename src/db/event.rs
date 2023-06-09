use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::str::FromStr;
use thiserror::Error;
use url::Url;

use crate::{
    db::DbRelayResponse,
    utils::{handle_decode_error, millis_to_naive_or_err, ns_event_to_millis, url_or_err},
};
use nostr::{
    secp256k1::{schnorr::Signature, XOnlyPublicKey},
    EventId, Kind, Tag, Timestamp,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("No tags found in the event")]
    NoTags,

    #[error("Wrong tag type found")]
    WrongTag,

    #[error("Event not in database: {0}")]
    EventNotInDatabase(EventId),

    #[error("Event already in database: {0}")]
    EventAlreadyInDatabase(EventId),

    #[error("{0}")]
    UtilsError(#[from] crate::utils::Error),

    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("JSON (de)serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Out-of-range number of seconds when parsing timestamp")]
    TimestampError,

    #[error("{0}")]
    FromDbRelayResponseError(#[from] crate::db::relay_response::Error),
}

#[derive(Debug, Clone)]
pub struct DbEvent {
    pub event_id: i64,
    pub event_hash: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: NaiveDateTime,
    pub relay_url: nostr::Url,
    pub tags: Vec<nostr::Tag>,
    pub kind: Kind,
    pub content: String,
    pub sig: Signature,
}

impl DbEvent {
    const FETCH_QUERY: &'static str = "SELECT * FROM event";

    pub fn to_ns_event(&self) -> Result<nostr::Event, Error> {
        Ok(nostr::Event {
            id: self.event_hash,
            pubkey: self.pubkey,
            created_at: Timestamp::from(self.created_at.timestamp() as u64),
            tags: self.tags.to_owned(),
            kind: self.kind,
            content: self.content.to_owned(),
            sig: self.sig,
        })
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbEvent>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let output = sqlx::query_as::<_, DbEvent>(&sql).fetch_all(pool).await?;
        Ok(output)
    }
    pub async fn fetch_kind(pool: &SqlitePool, kind: nostr::Kind) -> Result<Vec<DbEvent>, Error> {
        let sql = format!("{} WHERE kind = ?", Self::FETCH_QUERY);
        let output = sqlx::query_as::<_, DbEvent>(&sql)
            .bind(kind.as_u32())
            .fetch_all(pool)
            .await?;
        Ok(output)
    }

    pub async fn fetch_id(pool: &SqlitePool, event_id: i64) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} WHERE event_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(event_id)
            .fetch_optional(pool)
            .await?)
    }

    pub async fn fetch_hash(
        pool: &SqlitePool,
        event_hash: &EventId,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} WHERE event_hash = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(event_hash.to_string())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn has_event(pool: &SqlitePool, event_hash: &EventId) -> Result<bool, Error> {
        Ok(Self::fetch_hash(pool, event_hash).await?.is_some())
    }

    //fetch last event from db
    pub async fn fetch_last(pool: &SqlitePool) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} ORDER BY event_id DESC LIMIT 1", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .fetch_optional(pool)
            .await?)
    }

    pub(crate) async fn fetch_last_url(
        pool: &SqlitePool,
        url: &Url,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!(
            "{} WHERE relay_url = ? ORDER BY event_id DESC LIMIT 1",
            Self::FETCH_QUERY
        );
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(url.to_string())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn fetch_last_kind(
        pool: &SqlitePool,
        kind: nostr::Kind,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!(
            "{} WHERE kind = ? ORDER BY event_id DESC LIMIT 1",
            Self::FETCH_QUERY
        );
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(kind.as_u32())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn fetch_last_kind_pubkey(
        pool: &SqlitePool,
        kind: nostr::Kind,
        pubkey: &XOnlyPublicKey,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!(
            "{} WHERE kind = ? AND pubkey = ? ORDER BY event_id DESC LIMIT 1",
            Self::FETCH_QUERY
        );
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(kind.as_u32())
            .bind(pubkey.to_string())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(
        pool: &SqlitePool,
        relay_url: &Url,
        ns_event: &nostr::Event,
    ) -> Result<Option<DbEvent>, Error> {
        if Self::has_event(pool, &ns_event.id).await? {
            return Ok(None);
        }

        tracing::debug!("inserting event id: {}", ns_event.id);
        tracing::trace!("inserting event {:?}", ns_event);
        let sql = r#"
            INSERT INTO event
                (event_hash, pubkey, kind, content, sig, 
                    tags, relay_url, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#;

        let inserted = sqlx::query(sql)
            .bind(&ns_event.id.to_string())
            .bind(&ns_event.pubkey.to_string())
            .bind(&ns_event.kind.as_u32())
            .bind(&ns_event.content)
            .bind(&ns_event.sig.to_string())
            .bind(&serde_json::to_string(&ns_event.tags)?)
            .bind(&relay_url.to_string())
            .bind(&ns_event_to_millis(ns_event.created_at))
            .execute(pool)
            .await?;

        let db_event = Self::fetch_id(pool, inserted.last_insert_rowid())
            .await?
            .ok_or(Error::EventNotInDatabase(ns_event.id.to_owned()))?;

        DbRelayResponse::insert_ok(pool, relay_url, &db_event).await?;

        Ok(Some(db_event))
    }

    pub async fn delete(pool: &SqlitePool, event_id: i64) -> Result<(), Error> {
        tracing::info!("Deleting event with id {}", event_id);
        let sql = "DELETE FROM event WHERE event_id = ?";

        sqlx::query(sql).bind(event_id).execute(pool).await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbEvent {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let hex_hash = row.try_get::<String, &str>("event_hash")?;
        let event_hash =
            EventId::from_hex(hex_hash).map_err(|e| handle_decode_error(e, "event_hash"))?;

        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let pubkey =
            XOnlyPublicKey::from_str(&pubkey).map_err(|e| handle_decode_error(e, "pubkey"))?;

        let kind = row.try_get::<u32, &str>("kind")?;
        let kind = Kind::from(kind as u64);

        let sig = row.try_get::<String, &str>("sig")?;
        let sig = Signature::from_str(&sig).map_err(|e| handle_decode_error(e, "sig"))?;

        let tags = {
            let raw_str = row.try_get::<String, &str>("tags")?;
            let serialized_values: Vec<Vec<String>> =
                serde_json::from_str(&raw_str).map_err(|e| handle_decode_error(e, "tags"))?;

            let tags_result: Result<Vec<Tag>, _> =
                serialized_values.into_iter().map(Tag::parse).collect();

            let tags = tags_result.map_err(|e| handle_decode_error(e, "tags"))?;

            tags
        };

        let relay_url: String = row.try_get("relay_url")?;
        let relay_url = url_or_err(&relay_url, "relay_url")?;

        let created_at: i64 = row.try_get("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        Ok(DbEvent {
            event_id: row.try_get::<i64, &str>("event_id")?,
            created_at,
            event_hash,
            pubkey,
            relay_url,
            kind,
            tags,
            content: row.try_get::<String, &str>("content")?,
            sig,
        })
    }
}
