use std::str::FromStr;

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::error::Error;
use nostr_sdk::{
    secp256k1::{schnorr::Signature, XOnlyPublicKey},
    Event, EventId, Kind, Timestamp,
};

#[derive(Debug, Clone)]
pub struct DbEvent {
    pub event_hash: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: Timestamp,
    pub kind: Kind,
    pub content: String,
    pub sig: Signature,
}

impl DbEvent {
    pub fn new(event: Event) -> DbEvent {
        DbEvent {
            event_hash: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            content: event.content,
            sig: event.sig,
        }
    }

    const FETCH_QUERY: &'static str =
        "SELECT event_hash, pubkey, created_at, kind, content, sig FROM event";

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
        event_id: &EventId,
    ) -> Result<Option<DbEvent>, Error> {
        let sql = format!("{} WHERE event_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbEvent>(&sql)
            .bind(event_id.to_hex())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, event: DbEvent) -> Result<u64, Error> {
        let sql =
            "INSERT OR IGNORE INTO event (event_hash, pubkey, created_at, kind, content, sig) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

        let inserted = sqlx::query(sql)
            .bind(&event.event_hash.to_hex())
            .bind(&event.pubkey.to_string())
            .bind(&event.created_at.as_i64())
            .bind(&event.kind.as_u32())
            .bind(&event.content)
            .bind(&event.sig.to_string())
            .execute(pool)
            .await?;

        Ok(inserted.rows_affected())
    }

    pub async fn update(pool: &SqlitePool, event: DbEvent) -> Result<(), Error> {
        let sql =
            "UPDATE event SET pubkey=?, created_at=?, kind=?, content=?, sig=? WHERE event_hash=?";

        sqlx::query(sql)
            .bind(&event.pubkey.to_string())
            .bind(&event.created_at.as_i64())
            .bind(&event.kind.as_u32())
            .bind(&event.content)
            .bind(&event.sig.to_string())
            .bind(&event.event_hash.to_hex())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, event_id: EventId) -> Result<(), Error> {
        let sql = "DELETE FROM event WHERE event_hash=?";

        sqlx::query(sql)
            .bind(&event_id.to_hex())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl From<nostr_sdk::Event> for DbEvent {
    fn from(event: nostr_sdk::Event) -> Self {
        event.into()
    }
}

impl From<&nostr_sdk::Event> for DbEvent {
    fn from(event: &nostr_sdk::Event) -> Self {
        Self {
            event_hash: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            content: event.content.clone(),
            sig: event.sig,
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbEvent {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let hex_hash = row.try_get::<String, &str>("event_hash")?;
        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let created_at = row.try_get::<i64, &str>("created_at")?;
        let kind = row.try_get::<u32, &str>("kind")?;
        let sig = row.try_get::<String, &str>("sig")?;
        Ok(DbEvent {
            event_hash: EventId::from_hex(hex_hash).map_err(|e| sqlx::Error::ColumnDecode {
                index: "event_hash".into(),
                source: Box::new(e),
            })?,
            pubkey: XOnlyPublicKey::from_str(&pubkey).map_err(|e| sqlx::Error::ColumnDecode {
                index: "pubkey".into(),
                source: Box::new(e),
            })?,
            created_at: Timestamp::from(created_at as u64),
            kind: Kind::from(kind as u64),
            content: row.try_get::<String, &str>("content")?,
            sig: Signature::from_str(&sig).map_err(|e| sqlx::Error::ColumnDecode {
                index: "sig".into(),
                source: Box::new(e),
            })?,
        })
    }
}
