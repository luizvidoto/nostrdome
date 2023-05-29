use chrono::{NaiveDateTime, Utc};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::str::FromStr;
use thiserror::Error;

use crate::utils::{handle_decode_error, millis_to_naive_or_err, ns_event_to_naive, url_or_err};
use nostr::{
    secp256k1::{schnorr::Signature, XOnlyPublicKey},
    EventId, Kind, Tag, Timestamp,
};

use super::UserConfig;

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
}

#[derive(Debug, Clone)]
pub struct DbEvent {
    event_id: Option<i64>,
    pub event_hash: EventId,
    pub pubkey: XOnlyPublicKey,
    pub local_creation: NaiveDateTime, // this is the time when the event was created at the app
    pub received_at: Option<NaiveDateTime>, // this is the time when the event was received at the app
    remote_creation: Option<NaiveDateTime>, // this is the time that the relay says the event was created at
    pub relay_url: Option<nostr::Url>,
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
            created_at: Timestamp::from(
                self.remote_creation
                    .ok_or(Error::EventNotInDatabase(self.event_hash.clone()))?
                    .timestamp() as u64,
            ),
            tags: self.tags.to_owned(),
            kind: self.kind,
            content: self.content.to_owned(),
            sig: self.sig,
        })
    }

    // when the app creates the event, there is no relay_url
    // when it receives from a relay there is
    fn from_event(event: nostr::Event, relay_url: Option<&nostr::Url>) -> Result<Self, Error> {
        let (received_at, remote_creation, relay_url) = if let Some(relay_url) = relay_url {
            // the difference between remote creation and confirmed at
            (
                Some(Utc::now().naive_utc()),
                Some(ns_event_to_naive(event.created_at)?),
                Some(relay_url),
            )
        } else {
            (None, None, None)
        };

        Ok(Self {
            event_id: None,
            event_hash: event.id,
            pubkey: event.pubkey,
            received_at: received_at,
            local_creation: Utc::now().naive_utc(),
            remote_creation,
            relay_url: relay_url.cloned(),
            tags: event.tags,
            kind: event.kind,
            content: event.content,
            sig: event.sig,
        })
    }

    pub fn confirmed_event(event: nostr::Event, relay_url: &nostr::Url) -> Result<Self, Error> {
        Ok(Self::from_event(event, Some(relay_url))?)
    }

    // pending event is not confirmed yet
    pub fn pending_event(event: nostr::Event) -> Result<Self, Error> {
        Ok(Self::from_event(event, None)?)
    }

    pub fn remote_creation(&self) -> Option<NaiveDateTime> {
        self.remote_creation
    }

    pub fn event_id(&self) -> Result<i64, Error> {
        self.event_id
            .ok_or(Error::EventNotInDatabase(self.event_hash.clone()))
    }

    pub fn with_id(mut self, id: i64) -> Self {
        self.event_id = Some(id);
        self
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

    pub async fn fetch_one(
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
        Ok(Self::fetch_one(pool, event_hash).await?.is_some())
    }

    //fetch last event from db
    pub async fn fetch_last(pool: &SqlitePool) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} ORDER BY event_id DESC LIMIT 1", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
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

    // maybe create insert confirmed and insert pending?
    pub async fn insert(pool: &SqlitePool, db_event: &DbEvent) -> Result<(i64, u8), Error> {
        if let Some(db_event) = Self::fetch_one(pool, &db_event.event_hash).await? {
            // ignore err
            // return Err(Error::EventAlreadyInDatabase(db_event.event_hash.clone()));
            return Ok((db_event.event_id()?, 0));
        }

        tracing::debug!("inserting event id: {}", db_event.event_hash);
        tracing::trace!("inserting event {:?}", db_event);
        let sql = r#"
            INSERT INTO 
                event (event_hash, pubkey, kind, content, sig, tags, 
                relay_url, local_creation, remote_creation, received_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;

        let inserted = sqlx::query(sql)
            .bind(&db_event.event_hash.to_string())
            .bind(&db_event.pubkey.to_string())
            .bind(&db_event.kind.as_u32())
            .bind(&db_event.content)
            .bind(&db_event.sig.to_string())
            .bind(&serde_json::to_string(&db_event.tags)?)
            .bind(&db_event.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&db_event.local_creation.timestamp_millis())
            .bind(
                &db_event
                    .remote_creation
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(
                &db_event
                    .received_at
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .execute(pool)
            .await?;

        Ok((
            inserted.last_insert_rowid() as i64,
            inserted.rows_affected() as u8,
        ))
    }

    pub async fn confirm_event(
        pool: &SqlitePool,
        relay_url: &nostr::Url,
        mut db_event: DbEvent,
    ) -> Result<DbEvent, Error> {
        tracing::info!("Confirming event");
        tracing::debug!("{:?}", &db_event);
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = r#"
            UPDATE event 
            SET received_at=?, remote_creation=?, relay_url=? 
            WHERE event_id=?
        "#;

        let event_id = db_event.event_id()?;
        db_event.received_at = Some(now_utc);
        db_event.remote_creation = Some(db_event.local_creation.clone());
        db_event.relay_url = Some(relay_url.clone());

        sqlx::query(sql)
            .bind(
                &db_event
                    .received_at
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(
                &db_event
                    .remote_creation
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&relay_url.to_string())
            .bind(&event_id)
            .execute(pool)
            .await?;
        Ok(db_event)
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

        let local_creation = row.try_get::<i64, &str>("local_creation")?;
        let local_creation = NaiveDateTime::from_timestamp_millis(local_creation).ok_or(
            handle_decode_error(Box::new(Error::TimestampError), "local_creation"),
        )?;

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

        let relay_url = row
            .get::<Option<String>, &str>("relay_url")
            .map(|s| url_or_err(&s, "relay_url"))
            .transpose()?;

        let remote_creation = row
            .get::<Option<i64>, &str>("remote_creation")
            .map(|n| millis_to_naive_or_err(n, "remote_creation"))
            .transpose()?;

        let received_at = row
            .get::<Option<i64>, &str>("received_at")
            .map(|n| millis_to_naive_or_err(n, "received_at"))
            .transpose()?;

        Ok(DbEvent {
            event_id: Some(row.try_get::<i64, &str>("event_id")?),
            event_hash,
            pubkey,
            local_creation,
            relay_url,
            received_at: received_at,
            remote_creation,
            kind,
            tags,
            content: row.try_get::<String, &str>("content")?,
            sig,
        })
    }
}
