use nostr::{EventId, Url};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, SqlitePool};
use std::result::Result as StdResult;
use thiserror::Error;

use crate::utils::{event_hash_or_err, url_or_err};

use super::DbEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbRelayResponse {
    pub id: Option<i64>,
    pub event_id: i64,
    pub event_hash: EventId,
    pub relay_url: Url,
    pub status: ResponseStatus,
}
impl DbRelayResponse {
    pub fn ok(event_id: i64, event_hash: &EventId, relay_url: &Url) -> Self {
        Self {
            id: None,
            event_id,
            event_hash: event_hash.to_owned(),
            relay_url: relay_url.to_owned(),
            status: ResponseStatus::from_bool(true, None),
        }
    }
    pub fn error(
        event_id: i64,
        event_hash: &EventId,
        relay_url: &Url,
        error_message: &str,
    ) -> Self {
        Self {
            id: None,
            event_id,
            event_hash: event_hash.to_owned(),
            relay_url: relay_url.to_owned(),
            status: ResponseStatus::from_bool(false, Some(error_message.to_owned())),
        }
    }
    pub async fn fetch_by_event(
        pool: &SqlitePool,
        event_id: i64,
    ) -> Result<Vec<DbRelayResponse>, Error> {
        let sql = r#"
            SELECT *
            FROM relay_response
            WHERE event_id = ?
        "#;

        let responses = sqlx::query_as::<_, DbRelayResponse>(sql)
            .bind(event_id)
            .fetch_all(pool)
            .await?;

        Ok(responses)
    }
    pub async fn fetch_one(
        pool: &SqlitePool,
        response: &DbRelayResponse,
    ) -> Result<Option<DbRelayResponse>, Error> {
        let sql = r#"
            SELECT *
            FROM relay_response
            WHERE event_id = ? AND relay_url = ?
        "#;

        let db_response = sqlx::query_as::<_, DbRelayResponse>(sql)
            .bind(response.event_id)
            .bind(&response.relay_url.to_string())
            .fetch_optional(pool)
            .await?;

        Ok(db_response)
    }
    async fn insert(pool: &SqlitePool, response: &DbRelayResponse) -> Result<(), Error> {
        tracing::trace!("Inserting relay response: {:?}", response);
        let (status, error_message) = response.status.to_bool();

        if let Some(_) = Self::fetch_one(pool, &response).await? {
            return Ok(());
        }

        let sql = r#"
            INSERT INTO relay_response (event_id, event_hash, relay_url, status, error_message)
            VALUES (?, ?, ?, ?, ?)
        "#;

        sqlx::query(sql)
            .bind(response.event_id)
            .bind(&response.event_hash.to_string())
            .bind(&response.relay_url.to_string())
            .bind(status)
            .bind(error_message)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn insert_ok(
        pool: &SqlitePool,
        relay_url: &nostr::Url,
        db_event: &DbEvent,
    ) -> Result<(), Error> {
        let event_id = db_event.event_id;
        let relay_response = DbRelayResponse::ok(event_id, &db_event.event_hash, relay_url);
        DbRelayResponse::insert(pool, &relay_response).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseStatus {
    Ok,
    Error(String),
}
impl ResponseStatus {
    pub fn from_bool(value: bool, error_message: Option<String>) -> Self {
        match value {
            true => Self::Ok,
            false => {
                Self::Error(error_message.unwrap_or("Relay Error Status: No error message".into()))
            }
        }
    }
    pub fn to_bool(&self) -> (bool, Option<String>) {
        match self {
            Self::Ok => (true, None),
            Self::Error(e) => (false, Some(e.to_string())),
        }
    }
}

impl FromRow<'_, SqliteRow> for DbRelayResponse {
    fn from_row(row: &'_ SqliteRow) -> StdResult<Self, sqlx::Error> {
        let event_hash = row.try_get::<String, &str>("event_hash")?;
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;
        let relay_url = row.try_get::<String, &str>("relay_url")?;
        let relay_url = url_or_err(&relay_url, "relay_url")?;
        let error_message = row.get::<Option<String>, &str>("error_message");
        let status = ResponseStatus::from_bool(row.try_get::<bool, &str>("status")?, error_message);
        Ok(DbRelayResponse {
            id: Some(row.try_get::<i64, &str>("relay_response_id")?),
            event_id: row.try_get::<i64, &str>("event_id")?,
            event_hash,
            relay_url,
            status,
        })
    }
}
