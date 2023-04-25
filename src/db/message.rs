use std::str::FromStr;

use crate::{
    error::{Error, FromDbEventError},
    utils::handle_decode_error,
};
use chrono::NaiveDateTime;
use nostr_sdk::{nips::nip04, prelude::UncheckedUrl, secp256k1::XOnlyPublicKey, Keys, Url};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::utils::millis_to_naive;

use super::DbEvent;

#[derive(Debug, Clone)]
pub struct DbMessage {
    pub msg_id: Option<i32>,
    pub encrypted_content: String,
    pub from_pubkey: XOnlyPublicKey,
    pub to_pubkey: XOnlyPublicKey,
    pub event_id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub status: MessageStatus,
    pub relay_url: Option<UncheckedUrl>,
}

impl DbMessage {
    const FETCH_QUERY: &'static str =
        "SELECT msg_id, content, from_pub, to_pub, event_id, created_at, updated_at, status, relay_url FROM message";

    pub fn is_local(&self, own_pubkey: &XOnlyPublicKey) -> bool {
        own_pubkey == &self.from_pubkey
    }
    pub fn is_unseen(&self) -> bool {
        self.status.is_unseen()
    }

    pub fn new_local(
        keys: &Keys,
        to_pubkey: &XOnlyPublicKey,
        message: &str,
    ) -> Result<Self, Error> {
        let secret_key = keys.secret_key()?;
        let encrypted_content = nip04::encrypt(&secret_key, &to_pubkey, message)
            .map_err(|e| Error::EncryptionError(e.to_string()))?;
        Ok(Self {
            msg_id: None,
            encrypted_content,
            from_pubkey: keys.public_key(),
            to_pubkey: to_pubkey.to_owned(),
            event_id: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            status: MessageStatus::Offline,
            relay_url: None,
        })
    }
    pub fn from_db_event(db_event: DbEvent, relay_url: Option<Url>) -> Result<Self, Error> {
        let (to_pub, event_id) = Self::extract_to_pub_and_event_id(&db_event)?;
        Ok(Self {
            msg_id: None,
            encrypted_content: db_event.content.to_owned(),
            from_pubkey: db_event.pubkey.clone(),
            to_pubkey: to_pub,
            event_id: Some(event_id),
            created_at: db_event.created_at,
            updated_at: db_event.created_at,
            status: MessageStatus::Delivered,
            relay_url: relay_url.map(|url| url.into()),
        })
    }
    pub fn decrypt_message(&self, keys: &Keys) -> Result<String, Error> {
        let secret_key = keys.secret_key()?;
        if self.is_local(&keys.public_key()) {
            nip04::decrypt(&secret_key, &self.to_pubkey, &self.encrypted_content)
                .map_err(|e| Error::DecryptionError(e.to_string()))
        } else {
            nip04::decrypt(&secret_key, &self.from_pubkey, &self.encrypted_content)
                .map_err(|e| Error::DecryptionError(e.to_string()))
        }
    }
    pub fn with_event(mut self, event_id: i32) -> Self {
        self.event_id = Some(event_id);
        self
    }

    fn extract_to_pub_and_event_id(
        db_event: &DbEvent,
    ) -> Result<(XOnlyPublicKey, i32), FromDbEventError> {
        let tag = db_event.tags.get(0).ok_or(FromDbEventError::NoTags)?;
        match tag {
            nostr_sdk::Tag::PubKey(to_pub, _url) => {
                let event_id = db_event.event_id.ok_or_else(|| {
                    FromDbEventError::EventNotInDatabase(db_event.event_hash.clone())
                })?;
                Ok((to_pub.clone(), event_id))
            }
            _ => Err(FromDbEventError::WrongTag),
        }
    }

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbMessage>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let messages = sqlx::query_as::<_, DbMessage>(&sql).fetch_all(pool).await?;

        Ok(messages)
    }

    pub async fn fetch_unseen_count(
        pool: &SqlitePool,
        pubkey: &XOnlyPublicKey,
    ) -> Result<u8, Error> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
            FROM message
            WHERE from_pub = ?1 AND (status = ?2 OR status = ?3)",
        )
        .bind(pubkey.to_string())
        .bind(MessageStatus::Offline.to_i32())
        .bind(MessageStatus::Delivered.to_i32())
        .fetch_one(pool)
        .await?;

        Ok(count as u8)
    }

    pub async fn fetch_chat(
        pool: &SqlitePool,
        from_pub: &XOnlyPublicKey,
        to_pub: &XOnlyPublicKey,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE (from_pub = ? AND to_pub = ?) OR (from_pub = ? AND to_pub = ?)
            ORDER BY created_at
        "#;

        let messages = sqlx::query_as::<_, DbMessage>(sql)
            .bind(from_pub.to_string())
            .bind(to_pub.to_string())
            .bind(to_pub.to_string())
            .bind(from_pub.to_string())
            .fetch_all(pool)
            .await?;

        Ok(messages)
    }

    pub async fn insert_message(pool: &SqlitePool, db_message: &DbMessage) -> Result<(), Error> {
        let sql = r#"
            INSERT OR IGNORE INTO message (content, from_pub, to_pub, event_id, created_at, updated_at, status, relay_url)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#;

        sqlx::query(sql)
            .bind(&db_message.encrypted_content)
            .bind(&db_message.from_pubkey.to_string())
            .bind(&db_message.to_pubkey.to_string())
            .bind(&db_message.event_id)
            .bind(&db_message.created_at.timestamp_millis())
            .bind(&db_message.updated_at.timestamp_millis())
            .bind(&db_message.status.to_i32())
            .bind(&db_message.relay_url.as_ref().map(|url| url.to_string()))
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_message_status(
        pool: &SqlitePool,
        db_message: &DbMessage,
    ) -> Result<(), Error> {
        if let Some(msg_id) = db_message.msg_id {
            let sql = r#"
            UPDATE message 
            SET status = ?1
            WHERE msg_id = ?2
            "#;

            sqlx::query(sql)
                .bind(&db_message.status.to_i32())
                .bind(&msg_id)
                .execute(pool)
                .await?;

            Ok(())
        } else {
            Err(Error::MessageNotInDatabase)
        }
    }
}

fn millis_to_naive_or_err(millis: i64, index: &str) -> Result<NaiveDateTime, sqlx::Error> {
    millis_to_naive(millis).ok_or_else(|| sqlx::Error::ColumnDecode {
        index: index.into(),
        source: Box::new(Error::InvalidDate(millis.to_string())),
    })
}

fn pubkey_or_err(pubkey_str: &str, index: &str) -> Result<XOnlyPublicKey, sqlx::Error> {
    XOnlyPublicKey::from_str(pubkey_str).map_err(|e| handle_decode_error(e, index))
}

impl sqlx::FromRow<'_, SqliteRow> for DbMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("created_at")?, "created_at")?;
        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;
        let from_pub = pubkey_or_err(&row.try_get::<String, &str>("from_pub")?, "from_pub")?;
        let to_pub = pubkey_or_err(&row.try_get::<String, &str>("to_pub")?, "to_pub")?;

        Ok(DbMessage {
            msg_id: row.get::<Option<i32>, &str>("msg_id"),
            event_id: row.get::<Option<i32>, &str>("event_id"),
            encrypted_content: row.try_get::<String, &str>("content")?,
            from_pubkey: from_pub,
            to_pubkey: to_pub,
            created_at,
            updated_at,
            status: MessageStatus::from_i32(row.try_get::<i32, &str>("status")?),
            relay_url: row
                .get::<Option<String>, &str>("relay_url")
                .map(|s| {
                    UncheckedUrl::from_str(&s).map_err(|e| handle_decode_error(e, "relay_url"))
                })
                .transpose()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MessageStatus {
    Offline = 0,
    Delivered = 1,
    Seen = 2,
}

impl MessageStatus {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => MessageStatus::Offline,
            1 => MessageStatus::Delivered,
            _ => MessageStatus::Seen,
        }
    }

    pub fn to_i32(self) -> i32 {
        self as i32
    }
    pub fn is_unseen(&self) -> bool {
        match self {
            MessageStatus::Offline => true,
            MessageStatus::Delivered => true,
            MessageStatus::Seen => false,
        }
    }
}
