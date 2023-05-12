use chrono::{NaiveDateTime, Utc};
use nostr_sdk::{prelude::UncheckedUrl, secp256k1::XOnlyPublicKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::borrow::Borrow;
use std::str::FromStr;
use std::{path::PathBuf, result::Result as StdResult};
use thiserror::Error;

use crate::db::UserConfig;
use crate::net::{get_png_image_path, ImageKind, ImageSize};
use crate::{
    types::{ChatMessage, RelayUrl},
    utils::{millis_to_naive_or_err, profile_meta_or_err, unchecked_url_or_err},
};

type Result<T> = std::result::Result<T, DbContactError>;

#[derive(Error, Debug)]
pub enum DbContactError {
    // General errors
    #[error("Invalid Public Key")]
    InvalidPublicKey,
    // General errors
    #[error("Invalid Relay Url: {0}")]
    InvalidRelayUrl(String),
    #[error("Not found contact with pubkey: {0}")]
    NotFoundContact(String),
    #[error("Other type of Tag")]
    TagToContactError,
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContactStatus {
    Unknown = 0,
    Known = 1,
}

impl From<u8> for ContactStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => ContactStatus::Unknown,
            _ => ContactStatus::Known,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbContact {
    pubkey: XOnlyPublicKey,
    relay_url: Option<UncheckedUrl>,
    petname: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    status: ContactStatus,
    unseen_messages: u8,
    last_message_content: Option<String>,
    last_message_date: Option<NaiveDateTime>,
    profile_meta: Option<nostr_sdk::Metadata>,
    profile_meta_last_update: Option<NaiveDateTime>,
    local_profile_image: Option<PathBuf>,
    local_banner_image: Option<PathBuf>,
}

impl From<&DbContact> for nostr_sdk::Contact {
    fn from(c: &DbContact) -> Self {
        Self {
            pk: c.pubkey.to_owned(),
            relay_url: c.relay_url.to_owned(),
            alias: c.petname.to_owned(),
        }
    }
}

impl PartialEq for DbContact {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}

impl DbContact {
    const FETCH_QUERY: &'static str = r#"SELECT * FROM contact"#;

    pub fn new(pubkey: &XOnlyPublicKey) -> Self {
        Self {
            pubkey: pubkey.clone(),
            relay_url: None,
            petname: None,
            status: ContactStatus::Unknown,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            unseen_messages: 0,
            last_message_content: None,
            last_message_date: None,
            profile_meta: None,
            profile_meta_last_update: None,
            local_banner_image: None,
            local_profile_image: None,
        }
    }

    pub fn from_tag(tag: &Tag) -> Result<Self> {
        match tag {
            Tag::PubKey(pk, relay_url) => {
                let mut contact = Self::new(pk);
                if let Some(relay_url) = relay_url {
                    contact = contact.with_unchekd_relay_url(relay_url);
                }
                Ok(contact)
            }
            Tag::ContactList {
                pk,
                relay_url,
                alias,
            } => {
                let mut contact = Self::new(pk);
                if let Some(relay_url) = relay_url {
                    contact = contact.with_unchekd_relay_url(relay_url);
                }

                if let Some(petname) = alias {
                    contact = contact.with_petname(&petname);
                }

                Ok(contact)
            }
            _ => Err(DbContactError::TagToContactError),
        }
    }

    pub fn pubkey(&self) -> &XOnlyPublicKey {
        &self.pubkey
    }
    pub fn from_str(pubkey: &str) -> Result<Self> {
        let pubkey =
            XOnlyPublicKey::from_str(pubkey).map_err(|_| DbContactError::InvalidPublicKey)?;
        Ok(Self::new(&pubkey))
    }
    pub fn new_from_submit(pubkey: &str, petname: &str, relay_url: &str) -> Result<Self> {
        let db_contact = Self::from_str(pubkey)?;
        let db_contact = Self::edit_contact(db_contact, petname, relay_url)?;
        Ok(db_contact)
    }
    pub fn edit_contact(
        mut db_contact: DbContact,
        petname: &str,
        relay_url: &str,
    ) -> Result<DbContact> {
        db_contact.petname = Some(petname.to_owned());

        if !relay_url.is_empty() {
            db_contact.update_relay_url(relay_url)?;
        } else {
            db_contact.relay_url = None;
        }

        Ok(db_contact)
    }
    pub fn get_petname(&self) -> Option<String> {
        self.petname.clone()
    }
    pub fn get_profile_meta(&self) -> Option<nostr_sdk::Metadata> {
        self.profile_meta.clone()
    }
    pub fn get_profile_meta_last_update(&self) -> Option<NaiveDateTime> {
        self.profile_meta_last_update.clone()
    }
    pub fn get_profile_name(&self) -> Option<String> {
        if let Some(metadata) = &self.profile_meta {
            if let Some(name) = &metadata.name {
                return Some(name.to_owned());
            }
        }
        None
    }
    pub fn get_display_name(&self) -> Option<String> {
        if let Some(metadata) = &self.profile_meta {
            if let Some(display_name) = &metadata.display_name {
                return Some(display_name.to_owned());
            }
        }
        None
    }
    pub fn get_relay_url(&self) -> Option<UncheckedUrl> {
        self.relay_url.clone()
    }
    pub fn last_message_content(&self) -> Option<String> {
        self.last_message_content.clone()
    }
    pub fn last_message_date(&self) -> Option<NaiveDateTime> {
        self.last_message_date.clone()
    }
    pub fn last_message_pair(&self) -> (Option<String>, Option<NaiveDateTime>) {
        (self.last_message_content(), self.last_message_date())
    }
    pub fn unseen_messages(&self) -> u8 {
        self.unseen_messages
    }

    pub fn with_profile_meta(
        mut self,
        meta: &nostr_sdk::Metadata,
        last_update: NaiveDateTime,
    ) -> Self {
        if let Some(previous_update) = self.profile_meta_last_update {
            if previous_update > last_update {
                tracing::warn!("Cant update profile metadata with older data");
                return self;
            }
        }
        self.profile_meta = Some(meta.clone());
        self.profile_meta_last_update = Some(last_update);
        self
    }
    pub fn with_local_profile_image(mut self, path: &PathBuf) -> Self {
        self.local_profile_image = Some(path.to_owned());
        self
    }
    pub fn with_local_banner_image(mut self, path: &PathBuf) -> Self {
        self.local_banner_image = Some(path.to_owned());
        self
    }
    pub fn with_unchekd_relay_url(mut self, relay_url: &UncheckedUrl) -> Self {
        self.relay_url = Some(relay_url.to_owned());
        self
    }
    pub fn with_relay_url(mut self, relay_url: &str) -> Result<Self> {
        let url = Self::parse_url(relay_url)?;
        self.relay_url = Some(url);
        Ok(self)
    }
    pub fn with_petname(mut self, petname: &str) -> Self {
        self.petname = Some(petname.to_owned());
        self
    }

    pub fn select_name(&self) -> String {
        if let Some(petname) = &self.get_petname() {
            if !petname.trim().is_empty() {
                return petname.to_owned();
            }
        }

        if let Some(display_name) = &self.get_display_name() {
            if !display_name.trim().is_empty() {
                return display_name.to_owned();
            }
        }

        if let Some(profile_name) = &self.get_profile_name() {
            if !profile_name.trim().is_empty() {
                return profile_name.to_owned();
            }
        }

        self.pubkey().to_string()
    }

    fn update_relay_url(&mut self, relay_url: &str) -> Result<()> {
        let url = Self::parse_url(relay_url)?;
        self.relay_url = Some(url);
        Ok(())
    }
    fn parse_url(url: &str) -> Result<UncheckedUrl> {
        RelayUrl::try_into_unchecked_url(url)
            .map_err(|_e| DbContactError::InvalidRelayUrl(url.to_owned()))
    }

    pub fn local_profile_image_str(&self) -> Option<String> {
        self.local_profile_image.as_ref().map(|path| {
            let path_str = path.to_str().unwrap_or("");
            path_str.to_string()
        })
    }

    pub fn profile_image_sized(&self, size: ImageSize) -> Option<PathBuf> {
        self.local_profile_image.as_ref().map(|path| {
            let image_path = get_png_image_path(path, ImageKind::Profile, size);
            image_path
        })
    }

    pub fn local_banner_image_str(&self) -> Option<String> {
        self.local_banner_image.as_ref().map(|path| {
            let path_str = path.to_str().unwrap_or("");
            path_str.to_string()
        })
    }

    pub async fn new_message(
        pool: &SqlitePool,
        mut db_contact: DbContact,
        chat_message: &ChatMessage,
    ) -> Result<DbContact> {
        // do not update unseen count here because we may be in the chat
        if Some(&chat_message.display_time) >= db_contact.last_message_date.as_ref() {
            let now_utc = UserConfig::get_corrected_time(pool)
                .await
                .unwrap_or(Utc::now().naive_utc());
            db_contact.updated_at = now_utc;
            db_contact.last_message_content = Some(chat_message.content.to_owned());
            db_contact.last_message_date = Some(chat_message.display_time);

            let sql = r#"
                UPDATE contact 
                SET updated_at=?, last_message_content=?, last_message_date=?
                WHERE pubkey=?
            "#;

            sqlx::query(sql)
                .bind(&db_contact.updated_at.timestamp_millis())
                .bind(&db_contact.last_message_content)
                .bind(
                    &db_contact
                        .last_message_date
                        .as_ref()
                        .map(|d| d.timestamp_millis()),
                )
                .bind(&db_contact.pubkey.to_string())
                .execute(pool)
                .await?;
        } else {
            tracing::info!("Can't update last_message with an older message.");
        }

        Ok(db_contact)
    }

    pub async fn add_to_unseen_count(
        pool: &SqlitePool,
        mut db_contact: DbContact,
    ) -> Result<DbContact> {
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        db_contact.unseen_messages += 1;

        let sql = r#"
                UPDATE contact
                SET updated_at = ?, unseen_messages = ?
                WHERE pubkey = ?
            "#;

        sqlx::query(sql)
            .bind(now_utc.timestamp_millis())
            .bind(&db_contact.unseen_messages)
            .bind(&db_contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(db_contact)
    }

    pub async fn update_unseen_count(
        pool: &SqlitePool,
        db_contact: &DbContact,
        count: u8,
    ) -> Result<DbContact> {
        tracing::info!("updated contact count: {}", count);
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());
        let sql = r#"
                UPDATE contact 
                SET updated_at=?, unseen_messages=?
                WHERE pubkey=?
            "#;

        sqlx::query(sql)
            .bind(now_utc.timestamp_millis())
            .bind(count)
            .bind(&db_contact.pubkey.to_string())
            .execute(pool)
            .await?;

        let db_contact_updated = Self::fetch_one(pool, &db_contact.pubkey).await?.ok_or(
            DbContactError::NotFoundContact(db_contact.pubkey().to_string()),
        )?;

        Ok(db_contact_updated)
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbContact>> {
        let rows: Vec<DbContact> = sqlx::query_as::<_, DbContact>(Self::FETCH_QUERY)
            .fetch_all(pool)
            .await?;

        Ok(rows)
    }

    pub async fn fetch_one(
        pool: &SqlitePool,
        pubkey: &XOnlyPublicKey,
    ) -> Result<Option<DbContact>> {
        let sql = format!("{} WHERE pubkey = ?", Self::FETCH_QUERY);

        Ok(sqlx::query_as::<_, DbContact>(&sql)
            .bind(&pubkey.to_string())
            .fetch_optional(pool)
            .await?)
    }

    async fn insert_single_contact(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        contact: &DbContact,
    ) -> Result<()> {
        tracing::info!("Inserting Contact");
        tracing::info!("{:?}", contact); //todo: replace with debug
        let sql = r#"
            INSERT INTO contact 
                (pubkey, relay_url, petname, status, 
                unseen_messages, created_at, updated_at, last_message_content, 
                last_message_date, profile_meta, profile_meta_last_update, 
                local_profile_image, local_banner_image) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#;

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&contact.petname)
            .bind(contact.status as u8)
            .bind(contact.unseen_messages)
            .bind(contact.created_at.timestamp_millis())
            .bind(contact.updated_at.timestamp_millis())
            .bind(&contact.last_message_content)
            .bind(
                contact
                    .last_message_date
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&contact.profile_meta.as_ref().map(|meta| meta.as_json()))
            .bind(
                contact
                    .profile_meta_last_update
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(contact.local_profile_image_str())
            .bind(contact.local_banner_image_str())
            .execute(tx)
            .await?;

        Ok(())
    }

    pub async fn insert(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        let mut tx = pool.begin().await?;

        Self::insert_single_contact(&mut tx, contact).await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn insert_batch<T: Borrow<DbContact>>(
        pool: &SqlitePool,
        contacts: &[T],
    ) -> Result<()> {
        tracing::info!("Inserting Batch of contacts");

        let mut tx = pool.begin().await?;

        for contact in contacts {
            let contact: &DbContact = contact.borrow();
            if let Err(e) = Self::insert_single_contact(&mut tx, contact).await {
                tracing::error!("Error inserting contact: {:?}", e);
            }
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn update_basic(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        tracing::info!("Updating Basic Contact {}", contact.pubkey().to_string());
        tracing::debug!("{:?}", contact); //todo: replace with debug
        let sql = r#"
            UPDATE contact 
            SET relay_url=?, petname=?
            WHERE pubkey=?
        "#;

        sqlx::query(sql)
            .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&contact.petname)
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        tracing::info!("Updating Contact {}", contact.pubkey().to_string());
        tracing::debug!("{:?}", contact); //todo: replace with debug
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = r#"
            UPDATE contact 
            SET relay_url=?, petname=?, 
                status=?, unseen_messages=?,  
                last_message_content=?, last_message_date=?,
                profile_meta=?, profile_meta_last_update=?, updated_at=?, local_profile_image=?, local_banner_image=?
            WHERE pubkey=?
        "#;

        sqlx::query(sql)
            .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&contact.petname)
            .bind(contact.status as u8)
            .bind(contact.unseen_messages)
            .bind(&contact.last_message_content)
            .bind(
                contact
                    .last_message_date
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(&contact.profile_meta.as_ref().map(|meta| meta.as_json()))
            .bind(
                contact
                    .profile_meta_last_update
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(now_utc.timestamp_millis())
            .bind(contact.local_profile_image_str())
            .bind(contact.local_banner_image_str())
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        let sql = "DELETE FROM contact WHERE pubkey=?";

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub(crate) async fn last_update_at(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        pubkey: &XOnlyPublicKey,
    ) -> Result<Option<NaiveDateTime>> {
        let sql = "SELECT updated_at FROM contact WHERE pubkey=?";
        let result = sqlx::query(sql)
            .bind(&pubkey.to_string())
            .fetch_one(pool)
            .await
            .ok();
        let date = result.map(|row| row.get::<i64, &str>("updated_at"));
        let date = date
            .map(|d| millis_to_naive_or_err(d, "updated_at"))
            .transpose()?;
        Ok(date)
    }

    pub(crate) async fn have_contact(pool: &SqlitePool, pubkey: &XOnlyPublicKey) -> Result<bool> {
        let sql = "SELECT pubkey FROM contact WHERE pubkey=?";
        let result = sqlx::query(sql)
            .bind(&pubkey.to_string())
            .fetch_one(pool)
            .await
            .ok();
        Ok(result.is_some())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbContact {
    fn from_row(row: &'_ SqliteRow) -> StdResult<Self, sqlx::Error> {
        let profile_meta: Option<String> = row.try_get("profile_meta")?;
        let profile_meta = profile_meta
            .as_ref()
            .map(|json| profile_meta_or_err(json, "profile_meta"))
            .transpose()?;

        let profile_meta_last_update = row.get::<Option<i64>, &str>("profile_meta_last_update");
        let profile_meta_last_update = profile_meta_last_update
            .as_ref()
            .map(|date| millis_to_naive_or_err(*date, "profile_meta_last_update"))
            .transpose()?;

        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let created_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("created_at")?, "created_at")?;
        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;

        let relay_url = row
            .get::<Option<String>, &str>("relay_url")
            .map(|url| unchecked_url_or_err(&url, "relay_url"))
            .transpose()?;

        let local_profile_image: Option<String> = row.get("local_profile_image");
        let local_profile_image = local_profile_image.map(|path| PathBuf::from(path));

        let local_banner_image: Option<String> = row.get("local_banner_image");
        let local_banner_image = local_banner_image.map(|path| PathBuf::from(path));

        let petname: Option<String> = row.get("petname");

        Ok(DbContact {
            profile_meta,
            profile_meta_last_update,
            local_profile_image,
            local_banner_image,
            pubkey: XOnlyPublicKey::from_str(&pubkey).map_err(|e| sqlx::Error::ColumnDecode {
                index: "pubkey".into(),
                source: Box::new(e),
            })?,
            created_at,
            updated_at,
            petname,
            relay_url,
            status: row.get::<u8, &str>("status").into(),
            unseen_messages: row.try_get::<i64, &str>("unseen_messages")? as u8,
            last_message_content: row.get::<Option<String>, &str>("last_message_content"),
            last_message_date: row
                .get::<Option<i64>, &str>("last_message_date")
                .map(|n| millis_to_naive_or_err(n, "last_message_date"))
                .transpose()?,
        })
    }
}
