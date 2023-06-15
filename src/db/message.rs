use super::{DbEvent, UserConfig};
use crate::{
    types::{ChatMessage, PendingEvent},
    utils::{
        event_hash_or_err, message_status_or_err, millis_to_naive_or_err, public_key_or_err,
        url_or_err,
    },
};
use chrono::{NaiveDateTime, Utc};
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
    SqlxError(#[from] sqlx::Error),

    #[error("Unkown chat message: from_pubkey:{0} - to_pubkey:{1}")]
    UnknownChatMessage(String, String),

    #[error("{0}")]
    FromDbEventError(#[from] super::event::Error),

    #[error("Can't update message without id")]
    MessageNotInDatabase,

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("Decryption Error: {0}")]
    DecryptionError(String),

    #[error("{0}")]
    FromKeysError(#[from] nostr::key::Error),

    #[error("Not found pending message")]
    NotFoundPendingMessage,

    #[error("Not found confirmed message: {0}")]
    NotFoundMessage(EventId),

    #[error("Unknown message status: {0}")]
    UnknownStatus(i32),
}

pub struct TagInfo {
    pub from_pubkey: XOnlyPublicKey,
    pub to_pubkey: XOnlyPublicKey,
}

impl TagInfo {
    pub fn from_db_event(db_event: &DbEvent) -> Result<Self, Error> {
        if db_event.tags.is_empty() {
            return Err(Error::NoTags(db_event.event_hash.to_owned()));
        }

        for tag in &db_event.tags {
            match tag {
                nostr::Tag::PubKey(to_pub, _url) => {
                    return Ok(Self {
                        from_pubkey: db_event.pubkey,
                        to_pubkey: to_pub.clone(),
                    });
                }
                _ => (),
            }
        }

        Err(Error::NotFoundTag(db_event.event_hash.to_owned()))
    }
    pub fn contact_pubkey(&self, keys: &Keys) -> Result<XOnlyPublicKey, Error> {
        let user_pubkey = &keys.public_key();

        if user_pubkey == &self.from_pubkey {
            Ok(self.to_pubkey.to_owned())
        } else if user_pubkey == &self.to_pubkey {
            Ok(self.from_pubkey.to_owned())
        } else {
            Err(Error::UnknownChatMessage(
                self.from_pubkey.to_string(),
                self.to_pubkey.to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationInfo {
    pub confirmed_at: chrono::NaiveDateTime,
    pub relay_url: nostr::Url,
    pub event_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub id: i64,
    pub event_hash: EventId,
    pub encrypted_content: String,
    pub chat_pubkey: XOnlyPublicKey,
    pub is_users: bool,
    pub created_at: chrono::NaiveDateTime,
    pub status: MessageStatus,
    pub confirmation_info: Option<ConfirmationInfo>,
}

impl DbMessage {
    const FETCH_QUERY: &'static str = "SELECT * FROM message";

    pub fn is_unseen(&self) -> bool {
        self.status.is_unseen()
    }

    pub fn decrypt_message(&self, keys: &Keys, tag_info: &TagInfo) -> Result<String, Error> {
        let users_secret_key = keys.secret_key()?;
        if self.is_users {
            nip04::decrypt(
                &users_secret_key,
                &tag_info.to_pubkey,
                &self.encrypted_content,
            )
            .map_err(|e| Error::DecryptionError(e.to_string()))
        } else {
            nip04::decrypt(
                &users_secret_key,
                &tag_info.from_pubkey,
                &self.encrypted_content,
            )
            .map_err(|e| Error::DecryptionError(e.to_string()))
        }
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbMessage>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let messages = sqlx::query_as::<_, DbMessage>(&sql).fetch_all(pool).await?;
        Ok(messages)
    }
    pub async fn fetch_by_hash(
        pool: &SqlitePool,
        event_hash: &EventId,
    ) -> Result<Option<DbMessage>, Error> {
        let sql = format!("{} WHERE event_hash = ?", Self::FETCH_QUERY);
        let db_message_opt = sqlx::query_as::<_, DbMessage>(&sql)
            .bind(&event_hash.to_string())
            .fetch_optional(pool)
            .await?;
        Ok(db_message_opt)
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
    pub async fn fetch_one(pool: &SqlitePool, msg_id: i64) -> Result<Option<DbMessage>, Error> {
        let sql = format!("{} WHERE msg_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbMessage>(&sql)
            .bind(msg_id)
            .fetch_optional(pool)
            .await?)
    }
    pub async fn fetch_unseen_chat_count(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<i64, Error> {
        let sql = r#"
            SELECT COUNT(*)
            FROM message
            WHERE contact_pubkey = ? AND status = ?
        "#;

        let count: (i64,) = sqlx::query_as(sql)
            .bind(&contact_pubkey.to_string())
            .bind(MessageStatus::Delivered.to_i32())
            .fetch_one(pool)
            .await?;

        Ok(count.0)
    }

    pub async fn fetch_chat(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE contact_pubkey=?
            ORDER BY created_at DESC
            LIMIT 100
        "#;

        let messages = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&contact_pubkey.to_string())
            .fetch_all(pool)
            .await?;

        Ok(messages)
    }

    pub async fn fetch_chat_more(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
        first_message: ChatMessage,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE contact_pubkey=? and created_at < ? ORDER BY created_at DESC
            LIMIT 100
        "#;
        let messages = sqlx::query_as::<_, DbMessage>(&sql)
            .bind(&contact_pubkey.to_string())
            .bind(first_message.display_time.timestamp_millis())
            .fetch_all(pool)
            .await?;

        Ok(messages)
    }

    pub async fn fetch_chat_last(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<Option<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE contact_pubkey=?
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let message = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&contact_pubkey.to_string())
            .fetch_optional(pool)
            .await?;

        Ok(message)
    }

    pub async fn insert_pending(
        pool: &SqlitePool,
        pending_event: PendingEvent,
        contact_pubkey: &XOnlyPublicKey,
        is_users: bool,
    ) -> Result<Self, Error> {
        let ns_event = pending_event.ns_event();

        tracing::info!("Insert pending message. ID: {}", ns_event.id);

        let utc_now = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());
        let sql = r#"
            INSERT INTO message 
                (event_hash, content, contact_pubkey, is_users, created_at, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#;

        let output = sqlx::query(sql)
            .bind(&ns_event.id.to_string())
            .bind(&ns_event.content)
            .bind(&contact_pubkey.to_string())
            .bind(&is_users)
            .bind(&utc_now.timestamp_millis())
            .bind(&MessageStatus::Pending.to_i32())
            .execute(pool)
            .await?;

        let sql = "SELECT * FROM message WHERE msg_id = ?";
        let db_message = sqlx::query_as::<_, DbMessage>(sql)
            .bind(output.last_insert_rowid())
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFoundPendingMessage)?;

        Ok(db_message)
    }

    pub async fn insert_confirmed(
        pool: &SqlitePool,
        db_event: &DbEvent,
        contact_pubkey: &XOnlyPublicKey,
        is_users: bool,
    ) -> Result<DbMessage, Error> {
        tracing::debug!("Insert confirmed message. ID: {}", db_event.event_hash);

        match Self::fetch_by_hash(pool, &db_event.event_hash).await? {
            Some(mut db_message) => {
                tracing::debug!("Message already in database.  {:?}", &db_message);

                if db_message.confirmation_info.is_none() {
                    db_message = Self::confirm_message(pool, db_event).await?;
                }

                Ok(db_message)
            }
            None => {
                let sql = r#"
                    INSERT INTO message (content, contact_pubkey, is_users, created_at, 
                        status, event_hash, confirmed_at, event_id, relay_url)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);
                "#;

                let output = sqlx::query(sql)
                    .bind(&db_event.content)
                    .bind(&contact_pubkey.to_string())
                    .bind(&is_users)
                    .bind(&db_event.created_at.timestamp_millis())
                    .bind(&MessageStatus::Delivered.to_i32())
                    .bind(&db_event.event_hash.to_string())
                    .bind(&db_event.created_at.timestamp_millis())
                    .bind(&db_event.event_id)
                    .bind(&db_event.relay_url.to_string())
                    .execute(pool)
                    .await?;

                let sql = "SELECT * FROM message WHERE msg_id = ?";
                let db_message = sqlx::query_as::<_, DbMessage>(sql)
                    .bind(output.last_insert_rowid())
                    .fetch_optional(pool)
                    .await?
                    .ok_or(Error::NotFoundMessage(db_event.event_hash.to_owned()))?;

                Ok(db_message)
            }
        }
    }

    pub async fn confirm_message(
        pool: &SqlitePool,
        db_event: &DbEvent,
    ) -> Result<DbMessage, Error> {
        tracing::info!("Confirming message. ID: {}", db_event.event_hash);
        let mut db_message = Self::fetch_by_hash(pool, &db_event.event_hash)
            .await?
            .ok_or(Error::NotFoundMessage(db_event.event_hash.to_owned()))?;

        let utc_now = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        db_message.status = MessageStatus::Delivered;
        db_message.confirmation_info = Some(ConfirmationInfo {
            confirmed_at: utc_now,
            relay_url: db_event.relay_url.to_owned(),
            event_id: db_event.event_id.to_owned(),
        });

        let sql = r#"
            UPDATE message 
            SET status = ?, confirmed_at=?, relay_url=?, event_id=?
            WHERE event_hash = ?
        "#;

        sqlx::query(sql)
            .bind(&MessageStatus::Delivered.to_i32())
            .bind(&utc_now.timestamp_millis())
            .bind(&db_event.relay_url.to_string())
            .bind(&db_event.event_id)
            .bind(&db_event.event_hash.to_string())
            .execute(pool)
            .await?;

        Ok(db_message)
    }

    pub async fn message_seen(pool: &SqlitePool, db_message: &mut DbMessage) -> Result<(), Error> {
        let sql = "UPDATE message SET status = ?1 WHERE msg_id = ?2";

        sqlx::query(sql)
            .bind(&MessageStatus::Seen.to_i32())
            .bind(&db_message.id)
            .execute(pool)
            .await?;

        db_message.status = MessageStatus::Seen;

        Ok(())
    }

    pub(crate) fn display_time(&self) -> NaiveDateTime {
        self.confirmation_info
            .as_ref()
            .map(|c| c.confirmed_at.to_owned())
            .unwrap_or(self.created_at.to_owned())
    }

    pub(crate) async fn reset_unseen(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<(), Error> {
        let sql = r#"
            UPDATE message
            SET status = ?
            WHERE contact_pubkey = ? AND status = ?
        "#;
        sqlx::query(sql)
            .bind(MessageStatus::Seen.to_i32())
            .bind(&contact_pubkey.to_string())
            .bind(MessageStatus::Delivered.to_i32())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn mark_seen(pool: &SqlitePool, msg_id: i64) -> Result<(), Error> {
        let sql = r#"
            UPDATE message
            SET status = ?
            WHERE msg_id = ?
        "#;
        sqlx::query(sql)
            .bind(MessageStatus::Seen.to_i32())
            .bind(msg_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at = row.try_get::<i64, &str>("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        let status: i32 = row.try_get("status")?;
        let status = message_status_or_err(status, "status")?;

        let contact_pubkey = &row.try_get::<String, &str>("contact_pubkey")?;
        let contact_pubkey = public_key_or_err(&contact_pubkey, "contact_pubkey")?;

        let event_hash: String = row.try_get("event_hash")?;
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;

        let confirmed_at: Option<i64> = row.get("confirmed_at");
        let event_id: Option<i64> = row.get("event_id");
        let relay_url: Option<String> = row.get("relay_url");

        let confirmation_info = match (confirmed_at, relay_url, event_id) {
            (Some(confirmed_at), Some(relay_url), Some(event_id)) => {
                let confirmed_at = millis_to_naive_or_err(confirmed_at, "confirmed_at")?;
                let relay_url = url_or_err(&relay_url, "relay_url")?;

                Some(ConfirmationInfo {
                    confirmed_at,
                    relay_url,
                    event_id,
                })
            }
            _ => None,
        };

        Ok(DbMessage {
            id: row.try_get::<i64, &str>("msg_id")?,
            encrypted_content: row.try_get::<String, &str>("content")?,
            is_users: row.try_get::<bool, &str>("is_users")?,
            event_hash,
            chat_pubkey: contact_pubkey,
            created_at,
            status,
            confirmation_info,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending = 0,
    Delivered = 1,
    Seen = 2,
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
        self as i32
    }
    pub fn is_unseen(&self) -> bool {
        match self {
            MessageStatus::Pending => false,
            MessageStatus::Delivered => true,
            MessageStatus::Seen => false,
        }
    }
}
