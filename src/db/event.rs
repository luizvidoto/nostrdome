use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::error::Error;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Event, EventId, Kind, Timestamp};

#[derive(Debug, Clone)]
pub struct DbEvent {
    pub event_id: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: Timestamp,
    pub kind: Kind,
    pub content: String,
    pub ots: Option<String>,
}

impl DbEvent {
    pub fn new(event: Event) -> DbEvent {
        DbEvent {
            event_id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            content: event.content,
            ots: event.ots,
        }
    }

    const FETCH_QUERY: &'static str =
        "SELECT event_id, pubkey, created_at, kind, content, ots FROM event";

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

    pub async fn insert(pool: &SqlitePool, event: DbEvent) -> Result<(), Error> {
        let sql = "INSERT OR IGNORE INTO event (event_id, pubkey, created_at, kind, content, ots) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

        sqlx::query(sql)
            .bind(&event.event_id.to_hex())
            .bind(&event.pubkey.to_string())
            .bind(&event.created_at.as_i64())
            .bind(&event.kind.as_u32())
            .bind(&event.content)
            .bind(&event.ots)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, event: DbEvent) -> Result<(), Error> {
        let sql =
            "UPDATE event SET pubkey=?, created_at=?, kind=?, content=?, ots=? WHERE event_id=?";

        sqlx::query(sql)
            .bind(&event.pubkey.to_string())
            .bind(&event.created_at.as_i64())
            .bind(&event.kind.as_u32())
            .bind(&event.content)
            .bind(&event.ots)
            .bind(&event.event_id.to_hex())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, event_id: EventId) -> Result<(), Error> {
        let sql = "DELETE FROM event WHERE event_id=?";

        sqlx::query(sql)
            .bind(&event_id.to_hex())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbEvent {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let hex_ev_id = row.try_get::<String, &str>("event_id")?;
        let pubkey = row.try_get::<String, &str>("author")?;
        let created_at = row.try_get::<i64, &str>("created_at")?;
        let kind = row.try_get::<u32, &str>("kind")?;
        Ok(DbEvent {
            event_id: EventId::from_hex(hex_ev_id).map_err(|e| sqlx::Error::ColumnDecode {
                index: "event_id".into(),
                source: Box::new(e),
            })?,
            pubkey: XOnlyPublicKey::from_slice(pubkey.as_bytes()).map_err(|e| {
                sqlx::Error::ColumnDecode {
                    index: "author".into(),
                    source: Box::new(e),
                }
            })?,
            created_at: Timestamp::from(created_at as u64),
            kind: Kind::from(kind as u64),
            content: row.try_get::<String, &str>("content")?,
            ots: row.try_get::<Option<String>, &str>("ots")?,
        })
    }
}
