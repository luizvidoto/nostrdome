use chrono::{NaiveDateTime, Utc};
use nostr::EventId;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;

use crate::utils::{event_hash_or_err, millis_to_naive_or_err};

use super::UserConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Not found channel subscription: ID: {0}")]
    NotFoundChannelSubscription(EventId),
}

#[derive(Debug, Clone)]
pub struct ChannelSubscription {
    pub id: i64,
    pub channel_id: EventId,
    pub subscribed_at: NaiveDateTime,
}

impl ChannelSubscription {
    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<Self>, Error> {
        let sql = "SELECT * FROM channel_subscription ;";
        let channels = sqlx::query_as::<_, Self>(sql).fetch_all(pool).await?;

        Ok(channels)
    }

    pub async fn insert(pool: &SqlitePool, channel_id: &EventId) -> Result<Self, Error> {
        let utc_now = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = "INSERT INTO channel_subscription (channel_id, subscribed_at) VALUES (?, ?);";

        let output = sqlx::query(sql)
            .bind(channel_id.to_string())
            .bind(utc_now.timestamp_millis())
            .execute(pool)
            .await?;

        let sql = "SELECT * FROM channel_subscription WHERE id = ?";
        let channel = sqlx::query_as::<_, Self>(sql)
            .bind(output.last_insert_rowid())
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFoundChannelSubscription(channel_id.to_owned()))?;

        Ok(channel)
    }

    pub async fn delete(pool: &SqlitePool, channel_id: &EventId) -> Result<(), Error> {
        let sql = "DELETE FROM channel_subscription WHERE channel_id = ?;";

        sqlx::query(sql)
            .bind(channel_id.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for ChannelSubscription {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let subscribed_at = row.try_get::<i64, &str>("subscribed_at")?;
        let subscribed_at = millis_to_naive_or_err(subscribed_at, "subscribed_at")?;

        let channel_id: String = row.try_get("channel_id")?;
        let channel_id = event_hash_or_err(&channel_id, "channel_id")?;

        Ok(ChannelSubscription {
            id: row.try_get("id")?,
            channel_id,
            subscribed_at,
        })
    }
}
