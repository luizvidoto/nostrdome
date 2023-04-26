use std::str::FromStr;

use chrono::{NaiveDateTime, Utc};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::{Error, FromDbEventError},
    utils::{event_tt_to_naive, handle_decode_error, millis_to_naive_or_err},
};
use nostr_sdk::{
    secp256k1::{schnorr::Signature, XOnlyPublicKey},
    Event, EventId, Kind, Tag,
};

#[derive(Debug, Clone)]
pub struct DbEvent {
    event_id: Option<i64>,
    pub event_hash: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: NaiveDateTime,
    pub confirmed_at: Option<NaiveDateTime>,
    created_at_from_relay: NaiveDateTime,
    pub tags: Vec<nostr_sdk::Tag>,
    pub kind: Kind,
    pub content: String,
    pub sig: Signature,
    pub confirmed: bool,
}

impl DbEvent {
    const FETCH_QUERY: &'static str =
        "SELECT event_id, event_hash, pubkey, created_at, kind, content, sig, tags, confirmed, confirmed_at FROM event";

    fn from_event(event: Event, confirmed: bool) -> Result<Self, Error> {
        let created_at_from_relay = event_tt_to_naive(event.created_at)?;
        return Ok(DbEvent {
            event_id: None,
            event_hash: event.id,
            pubkey: event.pubkey,
            tags: event.tags.clone(),
            created_at: created_at_from_relay.clone(),
            confirmed_at: if confirmed {
                Some(created_at_from_relay.clone())
            } else {
                None
            },
            created_at_from_relay,
            kind: event.kind,
            content: event.content,
            sig: event.sig,
            confirmed,
        });
    }

    pub fn confirmed_event(event: Event) -> Result<Self, Error> {
        Ok(Self::from_event(event, true)?)
    }

    pub fn pending_event(event: Event) -> Result<Self, Error> {
        Ok(Self::from_event(event, false)?)
    }

    pub fn event_id(&self) -> Result<i64, FromDbEventError> {
        self.event_id.ok_or(FromDbEventError::EventNotInDatabase(
            self.event_hash.clone(),
        ))
    }

    pub fn with_id(mut self, id: i64) -> Self {
        self.event_id = Some(id);
        self
    }

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbEvent>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let output = sqlx::query_as::<_, DbEvent>(&sql).fetch_all(pool).await?;

        Ok(output)
    }

    pub async fn fetch_one(
        pool: &SqlitePool,
        event_hash: &EventId,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} WHERE event_hash = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(event_hash.to_hex())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, db_event: &DbEvent) -> Result<(i64, u8), Error> {
        let sql =
            "INSERT OR IGNORE INTO event (event_hash, pubkey, created_at, kind, content, sig, tags, created_at_from_relay, confirmed, confirmed_at) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";

        let inserted = sqlx::query(sql)
            .bind(&db_event.event_hash.to_hex())
            .bind(&db_event.pubkey.to_string())
            .bind(&db_event.created_at.timestamp_millis())
            .bind(&db_event.kind.as_u32())
            .bind(&db_event.content)
            .bind(&db_event.sig.to_string())
            .bind(&serde_json::to_string(&db_event.tags)?)
            .bind(&db_event.created_at_from_relay.timestamp_millis())
            .bind(&db_event.confirmed)
            .bind(
                &db_event
                    .confirmed_at
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

    pub async fn confirm_event(pool: &SqlitePool, mut db_event: DbEvent) -> Result<DbEvent, Error> {
        let sql = "UPDATE event SET confirmed_at=?, confirmed = 1 WHERE event_id=?";

        let event_id = db_event.event_id()?;
        db_event.confirmed_at = Some(Utc::now().naive_utc());

        sqlx::query(sql)
            .bind(
                &db_event
                    .confirmed_at
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&event_id)
            .execute(pool)
            .await?;

        Ok(db_event)
    }

    pub async fn delete(pool: &SqlitePool, event_id: i32) -> Result<(), Error> {
        let sql = "DELETE FROM event WHERE event_id = ?";

        sqlx::query(sql).bind(event_id).execute(pool).await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbEvent {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let hex_hash = row.try_get::<String, &str>("event_hash")?;
        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let created_at = row.try_get::<i64, &str>("created_at")?;
        let kind = row.try_get::<u32, &str>("kind")?;
        let sig = row.try_get::<String, &str>("sig")?;
        let tags = {
            let raw_str = row.try_get::<String, &str>("tags")?;
            let serialized_values: Vec<Vec<String>> =
                serde_json::from_str(&raw_str).map_err(|e| handle_decode_error(e, "tags"))?;

            let tags_result: Result<Vec<Tag>, _> =
                serialized_values.into_iter().map(Tag::parse).collect();

            let tags = tags_result.map_err(|e| handle_decode_error(e, "tags"))?;

            tags
        };

        Ok(DbEvent {
            confirmed_at: row
                .get::<Option<i64>, &str>("confirmed_at")
                .map(|n| millis_to_naive_or_err(n, "confirmed_at"))
                .transpose()?,
            event_id: Some(row.try_get::<i64, &str>("event_id")?),
            event_hash: EventId::from_hex(hex_hash)
                .map_err(|e| handle_decode_error(e, "event_hash"))?,
            pubkey: XOnlyPublicKey::from_str(&pubkey)
                .map_err(|e| handle_decode_error(e, "pubkey"))?,
            created_at: NaiveDateTime::from_timestamp_millis(created_at).ok_or(
                handle_decode_error(Box::new(Error::DbEventTimestampError), "created_at"),
            )?,
            created_at_from_relay: NaiveDateTime::from_timestamp_millis(created_at).ok_or(
                handle_decode_error(Box::new(Error::DbEventTimestampError), "relay_created_at"),
            )?,
            kind: Kind::from(kind as u64),
            tags,
            content: row.try_get::<String, &str>("content")?,
            sig: Signature::from_str(&sig).map_err(|e| handle_decode_error(e, "sig"))?,
            confirmed: row.try_get::<bool, &str>("confirmed")?,
        })
    }
}
