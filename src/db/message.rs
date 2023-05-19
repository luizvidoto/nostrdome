use crate::{
    error::{Error, FromDbEventError},
    utils::{event_hash_or_err, millis_to_naive_or_err, public_key_or_err, url_or_err},
};
use chrono::{NaiveDateTime, Utc};
use nostr_sdk::{nips::nip04, secp256k1::XOnlyPublicKey, EventId, Keys};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use super::{DbEvent, UserConfig};

pub struct TagInfo {
    pub from_pubkey: XOnlyPublicKey,
    pub to_pubkey: XOnlyPublicKey,
    pub event_id: i64,
    pub event_hash: EventId,
}

impl TagInfo {
    pub fn from_db_event(db_event: &DbEvent) -> Result<Self, FromDbEventError> {
        let tag = db_event.tags.get(0).ok_or(FromDbEventError::NoTags)?;
        match tag {
            nostr_sdk::Tag::PubKey(to_pub, _url) => Ok(Self {
                from_pubkey: db_event.pubkey,
                to_pubkey: to_pub.clone(),
                event_id: db_event.event_id()?,
                event_hash: db_event.event_hash,
            }),
            _ => Err(FromDbEventError::WrongTag),
        }
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
pub struct DbMessage {
    id: Option<i64>,
    encrypted_content: String,
    contact_pubkey: XOnlyPublicKey,
    from_pubkey: XOnlyPublicKey,
    to_pubkey: XOnlyPublicKey,
    created_at: chrono::NaiveDateTime,
    confirmed_at: Option<chrono::NaiveDateTime>,
    status: MessageStatus,
    relay_url: Option<nostr_sdk::Url>,
    // option because we can create the struct before inserting into the database
    event_id: Option<i64>,
    event_hash: Option<EventId>,
}

impl DbMessage {
    const FETCH_QUERY: &'static str = "SELECT * FROM message";

    pub fn im_author(&self, own_pubkey: &XOnlyPublicKey) -> bool {
        own_pubkey == &self.from_pubkey
    }
    pub fn is_unseen(&self) -> bool {
        self.status.is_unseen()
    }
    pub fn contact_chat(&self) -> XOnlyPublicKey {
        self.contact_pubkey.to_owned()
    }
    pub fn to_pubkey(&self) -> XOnlyPublicKey {
        self.to_pubkey.to_owned()
    }
    pub fn from_pubkey(&self) -> XOnlyPublicKey {
        self.from_pubkey.to_owned()
    }

    pub fn id(&self) -> Result<i64, Error> {
        Ok(self.id.ok_or(Error::MessageNotInDatabase)?)
    }
    pub fn event_id(&self) -> Result<i64, Error> {
        Ok(self.event_id.ok_or(Error::MessageNotInDatabase)?)
    }
    pub fn event_hash(&self) -> Result<EventId, Error> {
        Ok(self.event_hash.ok_or(Error::MessageNotInDatabase)?)
    }

    pub fn status(&self) -> MessageStatus {
        self.status.to_owned()
    }

    fn with_status(mut self, status: MessageStatus) -> Self {
        self.status = status;
        self
    }

    fn with_confirmed_at(mut self, confirmed_at: NaiveDateTime) -> Self {
        self.confirmed_at = Some(confirmed_at);
        self
    }

    pub fn with_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }
    pub fn with_event(mut self, event_id: i64) -> Self {
        self.event_id = Some(event_id);
        self
    }
    pub fn with_relay_url(mut self, relay_url: Option<&nostr_sdk::Url>) -> Self {
        self.relay_url = relay_url.cloned();
        self
    }

    pub fn new(db_event: &DbEvent, contact_pubkey: &XOnlyPublicKey) -> Result<Self, Error> {
        let info = TagInfo::from_db_event(&db_event)?;
        Ok(Self {
            id: None,
            contact_pubkey: contact_pubkey.to_owned(),
            encrypted_content: db_event.content.to_owned(),
            from_pubkey: info.from_pubkey,
            to_pubkey: info.to_pubkey,
            event_id: Some(info.event_id),
            event_hash: Some(info.event_hash),
            relay_url: None,
            status: MessageStatus::Offline,
            created_at: db_event.local_creation,
            confirmed_at: None,
        })
    }

    pub(crate) fn confirmed_message(
        db_event: &DbEvent,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<Self, Error> {
        let confirmed_at = db_event
            .remote_creation()
            .ok_or(Error::NotConfirmedEvent(db_event.event_hash.to_owned()))?;
        let msg = Self::new(db_event, contact_pubkey)?
            .with_relay_url(db_event.relay_url.as_ref())
            .with_status(MessageStatus::Delivered)
            .with_confirmed_at(confirmed_at);
        Ok(msg)
    }

    pub fn decrypt_message(&self, keys: &Keys) -> Result<String, Error> {
        let secret_key = keys.secret_key()?;
        if self.im_author(&keys.public_key()) {
            nip04::decrypt(&secret_key, &self.to_pubkey, &self.encrypted_content)
                .map_err(|e| Error::DecryptionError(e.to_string()))
        } else {
            nip04::decrypt(&secret_key, &self.from_pubkey, &self.encrypted_content)
                .map_err(|e| Error::DecryptionError(e.to_string()))
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
    pub async fn fetch_one(pool: &SqlitePool, msg_id: i64) -> Result<Option<DbMessage>, Error> {
        let sql = format!("{} WHERE msg_id = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbMessage>(&sql)
            .bind(msg_id)
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert_message(pool: &SqlitePool, db_message: &DbMessage) -> Result<i64, Error> {
        let sql = r#"
            INSERT OR IGNORE INTO message (content, contact_pubkey, from_pubkey, to_pubkey, created_at, confirmed_at, status, relay_url, event_id, event_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;

        let output = sqlx::query(sql)
            .bind(&db_message.encrypted_content)
            .bind(&db_message.contact_pubkey.to_string())
            .bind(&db_message.from_pubkey.to_string())
            .bind(&db_message.to_pubkey.to_string())
            .bind(&db_message.created_at.timestamp_millis())
            .bind(
                &db_message
                    .confirmed_at
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&db_message.status.to_i32())
            .bind(&db_message.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&db_message.event_id()?)
            .bind(&db_message.event_hash()?.to_hex())
            .execute(pool)
            .await?;

        Ok(output.last_insert_rowid())
    }

    pub async fn relay_confirmation(
        pool: &SqlitePool,
        relay_url: &nostr_sdk::Url,
        mut db_message: DbMessage,
    ) -> Result<DbMessage, Error> {
        tracing::debug!("Confirming message");
        tracing::debug!("{:?}", &db_message);
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = r#"
            UPDATE message 
            SET status = ?, confirmed_at=?, relay_url=?
            WHERE msg_id = ?
        "#;

        let msg_id = db_message.id()?;
        db_message.status = MessageStatus::Delivered;
        db_message.confirmed_at = Some(now_utc);
        db_message.relay_url = Some(relay_url.to_owned());

        sqlx::query(sql)
            .bind(&db_message.status.to_i32())
            .bind(
                &db_message
                    .confirmed_at
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&db_message.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&msg_id)
            .execute(pool)
            .await?;
        Ok(db_message)
    }

    pub async fn fetch_chat(
        pool: &SqlitePool,
        contact_pubkey: &XOnlyPublicKey,
    ) -> Result<Vec<DbMessage>, Error> {
        let sql = r#"
            SELECT *
            FROM message
            WHERE contact_pubkey=?
        "#;

        let messages = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&contact_pubkey.to_string())
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
            ORDER BY confirmed_at DESC
            LIMIT 1
        "#;

        let message = sqlx::query_as::<_, DbMessage>(sql)
            .bind(&contact_pubkey.to_string())
            .fetch_optional(pool)
            .await?;

        Ok(message)
    }

    pub async fn message_seen(pool: &SqlitePool, db_message: &mut DbMessage) -> Result<(), Error> {
        let sql = r#"
            UPDATE message 
            SET status = ?1
            WHERE msg_id = ?2
            "#;

        let msg_id = db_message.id()?;
        db_message.status = MessageStatus::Seen;

        sqlx::query(sql)
            .bind(&db_message.status.to_i32())
            .bind(&msg_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub(crate) fn display_time(&self) -> NaiveDateTime {
        self.confirmed_at.unwrap_or(self.created_at)
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbMessage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let confirmed_at = row.get::<Option<i64>, &str>("confirmed_at");
        let confirmed_at = confirmed_at
            .as_ref()
            .map(|date| millis_to_naive_or_err(*date, "confirmed_at"))
            .transpose()?;

        let created_at = row.try_get::<i64, &str>("created_at")?;
        let created_at = millis_to_naive_or_err(created_at, "created_at")?;

        let relay_url = row
            .get::<Option<String>, &str>("relay_url")
            .map(|url| url_or_err(&url, "relay_url"))
            .transpose()?;

        let contact_pubkey = public_key_or_err(
            &row.try_get::<String, &str>("contact_pubkey")?,
            "contact_pubkey",
        )?;
        let from_pubkey =
            public_key_or_err(&row.try_get::<String, &str>("from_pubkey")?, "from_pubkey")?;
        let to_pubkey = public_key_or_err(&row.try_get::<String, &str>("to_pubkey")?, "to_pubkey")?;

        let event_hash = row.try_get::<String, &str>("event_hash")?;
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;

        Ok(DbMessage {
            id: row.get::<Option<i64>, &str>("msg_id"),
            event_id: Some(row.try_get::<i64, &str>("event_id")?),
            encrypted_content: row.try_get::<String, &str>("content")?,
            contact_pubkey,
            from_pubkey,
            to_pubkey,
            created_at,
            confirmed_at,
            event_hash: Some(event_hash),
            status: MessageStatus::from_i32(row.try_get::<i32, &str>("status")?),
            relay_url,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
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
            MessageStatus::Offline => false,
            MessageStatus::Delivered => true,
            MessageStatus::Seen => false,
        }
    }

    pub(crate) fn is_offline(&self) -> bool {
        match self {
            MessageStatus::Offline => true,
            MessageStatus::Delivered => false,
            MessageStatus::Seen => false,
        }
    }
}
