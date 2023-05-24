#![allow(dead_code)]
use crate::{
    error::Error,
    utils::{event_hash_or_err, millis_to_naive_or_err},
};
use chrono::NaiveDateTime;
use nostr::EventId;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use super::DbEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbChannel {
    id: Option<i64>,
    pub event_id: i64,
    pub event_hash: EventId,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub channel_content: ChannelContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelContent {
    pub about: Option<String>,
    pub banner: Option<String>,
    pub display_name: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub website: Option<String>,
}
impl ChannelContent {
    pub fn empty() -> Self {
        Self {
            about: None,
            banner: None,
            display_name: None,
            name: None,
            picture: None,
            website: None,
        }
    }
}

impl DbChannel {
    const FETCH_QUERY: &'static str =
        "SELECT channel_id, event_id, event_hash, created_at, updated_at, about, banner, display_name, name, picture, website FROM channel";

    pub fn id(&self) -> Result<i64, Error> {
        Ok(self.id.ok_or(Error::ChannelNotInDatabase)?)
    }
    pub fn with_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn from_db_event(db_event: DbEvent) -> Result<Self, Error> {
        let channel_content = Self::info_from_tags(&db_event)?;
        Ok(Self {
            id: None,
            event_id: db_event.event_id()?,
            event_hash: db_event.event_hash,
            created_at: db_event.local_creation,
            updated_at: db_event.local_creation,
            channel_content,
        })
    }

    fn info_from_tags(_db_event: &DbEvent) -> Result<ChannelContent, Error> {
        // let tag = db_event.tags.get(0).ok_or(Error::NoTags)?;
        // match tag {
        //     nostr::Tag:: =>
        //     _ => Err(Error::WrongTag),
        // }
        Ok(ChannelContent::empty())
    }

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbChannel>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let messages = sqlx::query_as::<_, DbChannel>(&sql).fetch_all(pool).await?;

        Ok(messages)
    }

    pub async fn fetch_one(pool: &SqlitePool, event_id: i64) -> Result<Option<DbChannel>, Error> {
        let sql = format!("{} WHERE event_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbChannel>(&sql)
            .bind(event_id)
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, _db_channel: &DbChannel) -> Result<i64, Error> {
        let sql = r#"
            INSERT OR IGNORE INTO message ()
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#;

        let output = sqlx::query(sql)
            // .bind(&db_channel.encrypted_content)
            // .bind(&db_channel.from_pubkey.to_string())
            // .bind(&db_channel.to_pubkey.to_string())
            // .bind(&db_channel.created_at.timestamp_millis())
            // .bind(&db_channel.updated_at.timestamp_millis())
            // .bind(&db_channel.status.to_i32())
            // .bind(&db_channel.relay_url.as_ref().map(|url| url.to_string()))
            // .bind(&db_channel.event_id()?)
            // .bind(&db_channel.event_hash()?.to_hex())
            .execute(pool)
            .await?;

        Ok(output.last_insert_rowid())
    }

    pub async fn update(pool: &SqlitePool, db_channel: &DbChannel) -> Result<(), Error> {
        let sql = r#"
            UPDATE channel 
            SET status = ?1
            WHERE id = ?2
            "#;

        let channel_id = db_channel.id()?;

        sqlx::query(sql).bind(&channel_id).execute(pool).await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbChannel {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("created_at")?, "created_at")?;
        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;

        let event_hash = row.try_get::<String, &str>("event_hash")?;
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;

        let channel_content = ChannelContent::empty();

        Ok(DbChannel {
            id: row.get::<Option<i64>, &str>("id"),
            event_id: row.try_get::<i64, &str>("event_id")?,
            event_hash,
            created_at,
            updated_at,
            channel_content,
        })
    }
}
