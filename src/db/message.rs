use super::DbEvent;
use crate::utils::{message_status_or_err, millis_to_naive_or_err, public_key_or_err, url_or_err};
use chrono::NaiveDateTime;
use nostr::{nips::nip04, secp256k1::XOnlyPublicKey, EventId, Keys};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("No tags found in the event. ID: {0}")]
    NoTags(EventId),

    #[error("Not found tag. ID: {0}")]
    NotFoundTag(EventId),

    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Unkown chat message: from_pubkey:{0} - to_pubkey:{1}")]
    UnknownChatMessage(String, String),

    #[error("{0}")]
    FromDbEvent(#[from] super::event::Error),

    #[error("Can't update message without id")]
    MessageNotInDatabase,

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("Decryption Error: {0}")]
    Decryption(String),

    #[error("{0}")]
    FromKeys(#[from] nostr::key::Error),

    #[error("Not found pending message")]
    NotFoundPendingMessage,

    #[error("Not found confirmed message: {0}")]
    NotFoundMessage(EventId),

    #[error("Unknown message status: {0}")]
    UnknownStatus(i32),
}

pub struct MessageTagInfo {
    pub from_pubkey: XOnlyPublicKey,
    pub to_pubkey: XOnlyPublicKey,
}

impl MessageTagInfo {
    pub fn from_event_tags(
        event_hash: &EventId,
        event_pubkey: &XOnlyPublicKey,
        tags: &[nostr::Tag],
    ) -> Result<Self, Error> {
        for tag in tags {
            if let nostr::Tag::PubKey(to_pubkey, _url) = tag {
                return Ok(Self {
                    from_pubkey: event_pubkey.to_owned(),
                    to_pubkey: *to_pubkey,
                });
            }
        }

        Err(Error::NotFoundTag(event_hash.to_owned()))
    }

    /// Check which contact chat the message belongs and
    /// if it is from someone else, returns nothing
    pub fn chat_pubkey(&self, keys: &Keys) -> Option<XOnlyPublicKey> {
        let user_pubkey = &keys.public_key();

        if user_pubkey == &self.from_pubkey {
            Some(self.to_pubkey.to_owned())
        } else if user_pubkey == &self.to_pubkey {
            Some(self.from_pubkey.to_owned())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub event_id: i64,
    pub encrypted_content: String,
    pub chat_pubkey: XOnlyPublicKey,
    pub is_users: bool,
    pub created_at: chrono::NaiveDateTime,
    pub status: MessageStatus,
    pub relay_url: nostr::Url,
}

impl DbMessage {
    const FETCH_QUERY: &'static str = "SELECT * FROM message";

    pub fn is_unseen(&self) -> bool {
        self.status.is_unseen()
    }

    pub fn decrypt_message(&self, keys: &Keys, tag_info: &MessageTagInfo) -> Result<String, Error> {
        let users_secret_key = keys.secret_key()?;
        if self.is_users {
            nip04::decrypt(
                &users_secret_key,
                &tag_info.to_pubkey,
                &self.encrypted_content,
            )
            .map_err(|e| Error::Decryption(e.to_string()))
        } else {
            nip04::decrypt(
                &users_secret_key,
                &tag_info.from_pubkey,
                &self.encrypted_content,
            )
            .map_err(|e| Error::Decryption(e.to_string()))
        }
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbMessage>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let messages = sqlx::query_as::<_, DbMessage>(&sql).fetch_all(pool).await?;
        Ok(messages)
    }
    pub async fn fetch_by_event(
        pool: &SqlitePool,
        event_id: i64,
    ) -> Result<Option<DbMessage>, Error> {
        let sql = format!("{} WHERE event_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbMessage>(&sql)
            .bind(event_id)
            .fetch_optional(pool)
            .await?)
    }
    pub async fn fetch_unseen_chat_count(
        pool: &SqlitePool,
        chat_pubkey: &XOnlyPublicKey,
    ) -> Result<i64, Error> {
        let sql = r#"
            SELECT COUNT(*)
            FROM message
            WHERE chat_pubkey = ? AND status = ?
        "#;

        let count: (i64,) = sqlx::query_as(sql)
            .bind(&chat_pubkey.to_string())
            .bind(MessageStatus::Delivered.to_i32())
            .fetch_one(pool)
            .await?;

        Ok(count.0)
    }

    pub async fn fetch_chat(
        pool: &SqlitePool,
        chat_pubkey: &XOnlyPublicKey,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE chat_pubkey = ?
            ORDER BY created_at DESC
            LIMIT 100
        "#;

        let messages = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&chat_pubkey.to_string())
            .fetch_all(pool)
            .await?;

        Ok(messages)
    }

    pub async fn fetch_chat_more(
        pool: &SqlitePool,
        chat_pubkey: &XOnlyPublicKey,
        first_msg_date: NaiveDateTime,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE chat_pubkey=? and created_at < ? ORDER BY created_at DESC
            LIMIT 100
        "#;
        let messages = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&chat_pubkey.to_string())
            .bind(first_msg_date.timestamp_millis())
            .fetch_all(pool)
            .await?;

        Ok(messages)
    }

    pub async fn fetch_chat_last(
        pool: &SqlitePool,
        chat_pubkey: &XOnlyPublicKey,
    ) -> Result<Option<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE chat_pubkey=?
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let message = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&chat_pubkey.to_string())
            .fetch_optional(pool)
            .await?;

        Ok(message)
    }

    pub async fn insert_confirmed(
        pool: &SqlitePool,
        db_event: &DbEvent,
        chat_pubkey: &XOnlyPublicKey,
        is_users: bool,
    ) -> Result<DbMessage, Error> {
        tracing::debug!("Insert confirmed message. ID: {}", db_event.event_hash);

        match Self::fetch_by_event(pool, db_event.event_id).await? {
            Some(db_message) => {
                tracing::debug!("Message already in database.  {:?}", &db_message);
                Ok(db_message)
            }
            None => {
                let sql = r#"
                    INSERT INTO message 
                    (event_id, content, chat_pubkey, is_users, created_at, status, relay_url)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
                "#;

                sqlx::query(sql)
                    .bind(db_event.event_id)
                    .bind(&db_event.content)
                    .bind(chat_pubkey.to_string())
                    .bind(is_users)
                    .bind(db_event.created_at.timestamp_millis())
                    .bind(MessageStatus::Delivered.to_i32())
                    .bind(&db_event.relay_url.to_string())
                    .execute(pool)
                    .await?;

                let sql = "SELECT * FROM message WHERE event_id = ?;";
                let db_message = sqlx::query_as::<_, DbMessage>(sql)
                    .bind(db_event.event_id)
                    .fetch_optional(pool)
                    .await?
                    .ok_or(Error::NotFoundMessage(db_event.event_hash.to_owned()))?;

                Ok(db_message)
            }
        }
    }

    pub async fn message_seen(pool: &SqlitePool, db_message: &mut DbMessage) -> Result<(), Error> {
        let sql = "UPDATE message SET status = ?1 WHERE event_id = ?2";

        sqlx::query(sql)
            .bind(MessageStatus::Seen.to_i32())
            .bind(db_message.event_id)
            .execute(pool)
            .await?;

        db_message.status = MessageStatus::Seen;

        Ok(())
    }

    pub(crate) async fn reset_unseen(
        pool: &SqlitePool,
        chat_pubkey: &XOnlyPublicKey,
    ) -> Result<(), Error> {
        let sql = r#"
            UPDATE message
            SET status = ?
            WHERE chat_pubkey = ? AND status = ?
        "#;
        sqlx::query(sql)
            .bind(MessageStatus::Seen.to_i32())
            .bind(&chat_pubkey.to_string())
            .bind(MessageStatus::Delivered.to_i32())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn mark_seen(pool: &SqlitePool, event_id: i64) -> Result<(), Error> {
        let sql = r#"
            UPDATE message
            SET status = ?
            WHERE event_id = ?
        "#;
        sqlx::query(sql)
            .bind(MessageStatus::Seen.to_i32())
            .bind(event_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at: i64 = row.try_get("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        let status: i32 = row.try_get("status")?;
        let status = message_status_or_err(status, "status")?;

        let chat_pubkey: String = row.try_get("chat_pubkey")?;
        let chat_pubkey = public_key_or_err(&chat_pubkey, "chat_pubkey")?;

        let relay_url: String = row.try_get("relay_url")?;
        let relay_url = url_or_err(&relay_url, "relay_url")?;

        Ok(DbMessage {
            event_id: row.try_get::<i64, &str>("event_id")?,
            encrypted_content: row.try_get::<String, &str>("content")?,
            is_users: row.try_get::<bool, &str>("is_users")?,
            chat_pubkey,
            created_at,
            status,
            relay_url,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Delivered,
    Seen,
}

impl MessageStatus {
    pub fn from_i32(value: i32) -> Result<Self, Error> {
        match value {
            0 => Ok(MessageStatus::Pending),
            1 => Ok(MessageStatus::Delivered),
            2 => Ok(MessageStatus::Seen),
            value => Err(Error::UnknownStatus(value)),
        }
    }
    pub fn to_i32(self) -> i32 {
        match self {
            MessageStatus::Pending => 0,
            MessageStatus::Delivered => 1,
            MessageStatus::Seen => 2,
        }
    }
    pub fn is_unseen(&self) -> bool {
        match self {
            MessageStatus::Pending => false,
            MessageStatus::Delivered => true,
            MessageStatus::Seen => false,
        }
    }
}
