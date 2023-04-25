use std::str::FromStr;

use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::Error,
    utils::{event_tt_to_naive, handle_decode_error},
};
use nostr_sdk::{
    secp256k1::{schnorr::Signature, XOnlyPublicKey},
    Event, EventId, Kind, Tag,
};

#[derive(Debug, Clone)]
pub struct DbEvent {
    pub event_id: Option<i64>,
    pub event_hash: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: NaiveDateTime,
    pub tags: Vec<nostr_sdk::Tag>,
    pub kind: Kind,
    pub content: String,
    pub sig: Signature,
}

impl DbEvent {
    const FETCH_QUERY: &'static str =
        "SELECT event_hash, pubkey, created_at, kind, content, sig, tags FROM event";

    pub fn from_event(event: Event) -> Result<Self, Error> {
        let created_at = event_tt_to_naive(event.created_at);
        return Ok(DbEvent {
            event_id: None,
            event_hash: event.id,
            pubkey: event.pubkey,
            tags: event.tags.clone(),
            created_at,
            kind: event.kind,
            content: event.content,
            sig: event.sig,
        });
        // Err(Error::DbEventTimestampError)
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

    pub async fn insert(pool: &SqlitePool, event: &DbEvent) -> Result<(i64, u8), Error> {
        let sql =
            "INSERT OR IGNORE INTO event (event_hash, pubkey, created_at, kind, content, sig, tags) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

        let inserted = sqlx::query(sql)
            .bind(&event.event_hash.to_hex())
            .bind(&event.pubkey.to_string())
            .bind(&event.created_at.timestamp_millis())
            .bind(&event.kind.as_u32())
            .bind(&event.content)
            .bind(&event.sig.to_string())
            .bind(&serde_json::to_string(&event.tags)?)
            .execute(pool)
            .await?;

        Ok((
            inserted.last_insert_rowid() as i64,
            inserted.rows_affected() as u8,
        ))
    }

    pub async fn update(pool: &SqlitePool, event: DbEvent) -> Result<(), Error> {
        if let Some(event_id) = event.event_id {
            let sql =
            "UPDATE event SET pubkey=?, created_at=?, kind=?, content=?, sig=? WHERE event_id=?";

            sqlx::query(sql)
                .bind(&event.pubkey.to_string())
                .bind(&event.created_at.timestamp_millis())
                .bind(&event.kind.as_u32())
                .bind(&event.content)
                .bind(&event.sig.to_string())
                .bind(&event_id)
                .execute(pool)
                .await?;
            Ok(())
        } else {
            Err(Error::EventNotInDatabase(event.event_hash))
        }
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
            event_id: row.get("event_id"),
            event_hash: EventId::from_hex(hex_hash)
                .map_err(|e| handle_decode_error(e, "event_hash"))?,
            pubkey: XOnlyPublicKey::from_str(&pubkey)
                .map_err(|e| handle_decode_error(e, "pubkey"))?,
            created_at: NaiveDateTime::from_timestamp_millis(created_at).ok_or(
                handle_decode_error(Box::new(Error::DbEventTimestampError), "created_at"),
            )?,
            kind: Kind::from(kind as u64),
            tags,
            content: row.try_get::<String, &str>("content")?,
            sig: Signature::from_str(&sig).map_err(|e| handle_decode_error(e, "sig"))?,
        })
    }
}
