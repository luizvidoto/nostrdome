use crate::{crypto::encrypt_local_message, error::Error, utils::event_tt_to_naive};
use chrono::Utc;
use nostr_sdk::{
    secp256k1::{SecretKey, XOnlyPublicKey},
    Timestamp,
};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::utils::millis_to_naive;

#[derive(Debug, Clone)]
pub struct DbMessage {
    pub msg_id: Option<i32>,
    pub content: String,
    pub from_pub: String,
    pub to_pub: String,
    pub event_id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub status: MessageStatus,
}

impl DbMessage {
    const FETCH_QUERY: &'static str =
        "SELECT msg_id, content, from_pub, to_pub, event_id, created_at, updated_at, status FROM message";

    pub fn is_sent(&self, pubkey: &XOnlyPublicKey) -> bool {
        pubkey.to_string() == self.from_pub
    }

    pub fn new_local(
        from_pub: &XOnlyPublicKey,
        to_pub: &XOnlyPublicKey,
        secret_key: &SecretKey,
        message: &str,
    ) -> Result<Self, Error> {
        // let content = nostr_sdk::nips::nip04::encrypt(sk, pk, text)
        let content = encrypt_local_message(secret_key, message)?;
        Ok(Self {
            msg_id: None,
            content,
            from_pub: from_pub.to_string(),
            to_pub: to_pub.to_string(),
            event_id: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            status: MessageStatus::Offline,
        })
    }
    pub fn with_event(mut self, event_id: i32) -> Self {
        self.event_id = Some(event_id);
        self
    }

    pub fn from_db_event(
        event_id: i32,
        created_at: Timestamp,
        from_pub: &XOnlyPublicKey,
        to_pub: &XOnlyPublicKey,
        secret_key: &SecretKey,
        content: &str,
    ) -> Result<Self, Error> {
        Ok(Self {
            msg_id: None,
            content: content.to_owned(),
            from_pub: from_pub.to_string(),
            to_pub: to_pub.to_string(),
            event_id: Some(event_id),
            created_at: event_tt_to_naive(created_at).unwrap_or(Utc::now().naive_utc()),
            updated_at: event_tt_to_naive(created_at).unwrap_or(Utc::now().naive_utc()),
            status: MessageStatus::Delivered,
        })
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

    pub async fn insert_message(pool: &SqlitePool, message: &DbMessage) -> Result<(), Error> {
        let sql = r#"
            INSERT OR IGNORE INTO message (content, from_pub, to_pub, event_id, created_at, updated_at, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#;

        sqlx::query(sql)
            .bind(&message.content)
            .bind(&message.from_pub)
            .bind(&message.to_pub)
            .bind(&message.event_id)
            .bind(&message.created_at.timestamp_millis())
            .bind(&message.updated_at.timestamp_millis())
            .bind(&message.status.to_i32())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at_num = row.try_get::<i64, &str>("created_at")?;
        let created_at =
            millis_to_naive(created_at_num).ok_or_else(|| sqlx::Error::ColumnDecode {
                index: "created_at".into(),
                source: Box::new(Error::InvalidDate(created_at_num.to_string())),
            })?;
        let updated_at_num = row.try_get::<i64, &str>("updated_at")?;
        let updated_at =
            millis_to_naive(updated_at_num).ok_or_else(|| sqlx::Error::ColumnDecode {
                index: "updated_at".into(),
                source: Box::new(Error::InvalidDate(updated_at_num.to_string())),
            })?;
        Ok(DbMessage {
            msg_id: row.get::<Option<i32>, &str>("msg_id"),
            event_id: row.get::<Option<i32>, &str>("event_id"),
            content: row.try_get::<String, &str>("content")?,
            from_pub: row.try_get::<String, &str>("from_pub")?,
            to_pub: row.try_get::<String, &str>("to_pub")?,
            created_at,
            updated_at,
            status: MessageStatus::from_i32(row.try_get::<i32, &str>("status")?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MessageStatus {
    Offline = 0,
    Delivered = 1,
}

impl MessageStatus {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => MessageStatus::Offline,
            1..=i32::MAX => MessageStatus::Delivered,
        }
    }

    pub fn to_i32(self) -> i32 {
        self as i32
    }
}
