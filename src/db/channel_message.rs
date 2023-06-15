use chrono::NaiveDateTime;
use nostr::{prelude::ToBech32, secp256k1::XOnlyPublicKey, EventId};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;
use url::Url;

use crate::utils::{
    channel_id_from_tags, event_hash_or_err, millis_to_naive_or_err, public_key_or_err, url_or_err,
};

use super::DbEvent;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Not found channel id inside event tags: event_hash: {0}")]
    NotFoundChannelInTags(nostr::EventId),

    #[error("Not found channel message: event_hash: {0}")]
    NotFoundMessage(EventId),
}

#[derive(Debug, Clone)]
pub struct DbChannelMessage {
    pub event_id: i64,
    pub channel_id: EventId,
    pub author: XOnlyPublicKey,
    pub is_users: bool,
    pub created_at: NaiveDateTime,
    pub relay_url: Url,
    pub content: String,
}
impl DbChannelMessage {
    pub fn display_name(&self) -> String {
        self.author.to_bech32().unwrap_or(self.author.to_string())
    }

    pub async fn fetch_one(pool: &SqlitePool, event_id: i64) -> Result<Option<Self>, Error> {
        let sql = "SELECT * FROM channel_message WHERE event_id = ?;";
        let ch_message = sqlx::query_as::<_, Self>(sql)
            .bind(event_id)
            .fetch_optional(pool)
            .await?;

        Ok(ch_message)
    }

    pub async fn fetch(pool: &SqlitePool, channel_id: &EventId) -> Result<Vec<Self>, Error> {
        let sql = r#"
            SELECT * FROM channel_message 
            WHERE channel_id = ? 
            ORDER BY created_at ASC 
            LIMIT 100;
        "#;
        let messages = sqlx::query_as::<_, Self>(sql)
            .bind(channel_id.to_string())
            .fetch_all(pool)
            .await?;
        Ok(messages)
    }

    pub async fn insert_confirmed(
        pool: &SqlitePool,
        db_event: &DbEvent,
        is_users: bool,
    ) -> Result<Self, Error> {
        match Self::fetch_one(pool, db_event.event_id).await? {
            Some(ch_message) => {
                tracing::debug!("Message already in database.  {:?}", &ch_message);

                // if ch_message.confirmation_info.is_none() {
                //     ch_message = Self::confirm_message(pool, db_event).await?;
                // }

                Ok(ch_message)
            }
            None => {
                let channel_id = channel_id_from_tags(&db_event.tags)
                    .ok_or(Error::NotFoundChannelInTags(db_event.event_hash.to_owned()))?;

                let sql = r#"
                    INSERT INTO channel_message (
                        event_id, channel_id, author, is_users, created_at, relay_url, content
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
                "#;

                let output = sqlx::query(sql)
                    .bind(&db_event.event_id)
                    .bind(&channel_id.to_string())
                    .bind(&db_event.pubkey.to_string())
                    .bind(is_users)
                    .bind(&db_event.created_at.timestamp_millis())
                    .bind(&db_event.relay_url.to_string())
                    .bind(&db_event.content)
                    .execute(pool)
                    .await?;

                let sql = "SELECT * FROM channel_message WHERE event_id = ?";
                let db_message = sqlx::query_as::<_, Self>(sql)
                    .bind(output.last_insert_rowid())
                    .fetch_optional(pool)
                    .await?
                    .ok_or(Error::NotFoundMessage(db_event.event_hash.to_owned()))?;

                Ok(db_message)
            }
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbChannelMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at = row.try_get::<i64, &str>("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        let author = &row.try_get::<String, &str>("author")?;
        let author = public_key_or_err(&author, "author")?;

        let channel_id: String = row.try_get("channel_id")?;
        let channel_id = event_hash_or_err(&channel_id, "channel_id")?;

        // let event_id: Option<i64> = row.get("event_id");
        // let relay_url: Option<String> = row.get("relay_url");
        let event_id: i64 = row.try_get("event_id")?;
        let relay_url: String = row.try_get("relay_url")?;
        let relay_url = url_or_err(&relay_url, "relay_url")?;

        let content: String = row.try_get("content")?;
        let is_users: bool = row.try_get("is_users")?;

        Ok(DbChannelMessage {
            event_id,
            channel_id,
            is_users,
            author,
            created_at,
            relay_url,
            content,
        })
    }
}
