use crate::error::Error;
use sqlx::{sqlite::SqliteRow, Row};

use crate::utils::millis_to_naive;

#[derive(Debug, Clone)]
pub struct DbMessage {
    pub msg_id: i32,
    pub content: String,
    pub from_pub: String,
    pub to_pub: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub status: MessageStatus,
}

impl DbMessage {
    const _FETCH_QUERY: &'static str =
        "SELECT msg_id, content, from_pub, to_pub, created_at, updated_at, status FROM message";

    // Funções CRUD (create, read, update, delete) para DbMessage
    // fetch, fetch_one, insert, update, delete
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
            msg_id: row.try_get::<i32, &str>("msg_id")?,
            content: row.try_get::<String, &str>("content")?,
            from_pub: row.try_get::<String, &str>("from_pub")?,
            to_pub: row.try_get::<String, &str>("to_pub")?,
            created_at,
            updated_at,
            status: MessageStatus::from_i32(row.try_get::<i32, &str>("status")?)
                .unwrap_or(MessageStatus::Offline),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MessageStatus {
    Offline = 0,
    Delivered = 1,
}

impl MessageStatus {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(MessageStatus::Offline),
            1 => Some(MessageStatus::Delivered),
            _ => None,
        }
    }

    pub fn _to_i32(self) -> i32 {
        self as i32
    }
}
