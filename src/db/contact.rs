use chrono::{NaiveDateTime, Utc};
use iced::widget::image::Handle;
use nostr::prelude::FromBech32;
use nostr::{prelude::UncheckedUrl, secp256k1::XOnlyPublicKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::borrow::Borrow;
use std::str::FromStr;
use thiserror::Error;
use url::Url;

use crate::consts::default_profile_image;
use crate::db::UserConfig;
use crate::net::{self, sized_image, BackEndConnection, ImageKind, ImageSize};
use crate::{
    types::RelayUrl,
    utils::{millis_to_naive_or_err, unchecked_url_or_err},
};

use super::ProfileCache;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid Public Key")]
    InvalidPublicKey,

    #[error("Invalid Relay Url: {0}")]
    InvalidRelayUrl(String),

    #[error("Not found contact with pubkey: {0}")]
    NotFoundContact(String),

    #[error("Other type of Tag")]
    TagToContactError,

    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("{0}")]
    FromProfileCacheError(#[from] crate::db::profile_cache::Error),
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
    profile_cache: Option<ProfileCache>,
}

impl From<&DbContact> for nostr::Contact {
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
            profile_cache: None,
        }
    }

    pub fn from_tag(tag: &Tag) -> Result<Self, Error> {
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
            _ => Err(Error::TagToContactError),
        }
    }

    pub fn pubkey(&self) -> &XOnlyPublicKey {
        &self.pubkey
    }
    pub fn from_str(pubkey: &str) -> Result<Self, Error> {
        match XOnlyPublicKey::from_bech32(pubkey) {
            Ok(pubkey) => Ok(Self::new(&pubkey)),
            Err(_e) => {
                let pubkey =
                    XOnlyPublicKey::from_str(pubkey).map_err(|_| Error::InvalidPublicKey)?;
                Ok(Self::new(&pubkey))
            }
        }
    }
    pub fn new_from_submit(pubkey: &str, petname: &str, relay_url: &str) -> Result<Self, Error> {
        let db_contact = Self::from_str(pubkey)?;
        let db_contact = Self::edit_contact(db_contact, petname, relay_url)?;
        Ok(db_contact)
    }
    pub fn edit_contact(
        mut db_contact: DbContact,
        petname: &str,
        relay_url: &str,
    ) -> Result<DbContact, Error> {
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
    pub fn get_profile_cache(&self) -> Option<ProfileCache> {
        self.profile_cache.clone()
    }
    pub fn get_profile_pic(&self) -> Option<String> {
        self.profile_cache
            .as_ref()
            .map(|profile| profile.metadata.picture.clone())
            .flatten()
    }
    pub fn get_profile_name(&self) -> Option<String> {
        if let Some(profile) = &self.profile_cache {
            if let Some(name) = &profile.metadata.name {
                return Some(name.to_owned());
            }
        }
        None
    }
    pub fn get_display_name(&self) -> Option<String> {
        if let Some(profile) = &self.profile_cache {
            if let Some(display_name) = &profile.metadata.display_name {
                return Some(display_name.to_owned());
            }
        }
        None
    }
    pub fn get_relay_url(&self) -> Option<UncheckedUrl> {
        self.relay_url.clone()
    }

    pub fn with_profile_cache(mut self, cache: &ProfileCache) -> Self {
        self.profile_cache = Some(cache.clone());
        self
    }
    pub fn with_unchekd_relay_url(mut self, relay_url: &UncheckedUrl) -> Self {
        self.relay_url = Some(relay_url.to_owned());
        self
    }
    pub fn with_relay_url(mut self, relay_url: &str) -> Result<Self, Error> {
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

    fn update_relay_url(&mut self, relay_url: &str) -> Result<(), Error> {
        let url = Self::parse_url(relay_url)?;
        self.relay_url = Some(url);
        Ok(())
    }
    fn parse_url(url: &str) -> Result<UncheckedUrl, Error> {
        RelayUrl::try_into_unchecked_url(url).map_err(|_e| Error::InvalidRelayUrl(url.to_owned()))
    }

    pub fn profile_image(&self, size: ImageSize, conn: &mut BackEndConnection) -> Handle {
        if let Some(cache) = &self.profile_cache {
            let kind = ImageKind::Profile;
            if let Some(img_cache) = &cache.profile_pic_cache {
                let path = sized_image(&img_cache.path, kind, size);
                return Handle::from_path(path);
            } else {
                if let Some(image_url) = &cache.metadata.picture {
                    match Url::parse(image_url) {
                        Ok(image_url_parsed) => {
                            tracing::info!("Download image. url: {}", image_url);
                            conn.send(net::ToBackend::DownloadImage {
                                image_url: image_url_parsed,
                                kind,
                                identifier: self.pubkey.to_string(),
                            });
                        }
                        Err(e) => {
                            tracing::error!("Error parsing image url: {:?}", e);
                        }
                    }
                } else {
                    tracing::info!("Contact don't have profile image");
                }
            }
        } else {
            tracing::info!("no profile cache for contact: {}", self.pubkey.to_string());
        }

        Handle::from_memory(default_profile_image(size))
    }

    pub async fn fetch_basic(pool: &SqlitePool) -> Result<Vec<DbContact>, Error> {
        let db_contacts = sqlx::query_as::<_, DbContact>(Self::FETCH_QUERY)
            .fetch_all(pool)
            .await?;
        Ok(db_contacts)
    }

    pub async fn fetch(
        pool: &SqlitePool,
        cache_pool: &SqlitePool,
    ) -> Result<Vec<DbContact>, Error> {
        let mut db_contacts: Vec<DbContact> = sqlx::query_as::<_, DbContact>(Self::FETCH_QUERY)
            .fetch_all(pool)
            .await?;

        for mut db_contact in &mut db_contacts {
            if let Some(cache) =
                ProfileCache::fetch_by_public_key(cache_pool, db_contact.pubkey()).await?
            {
                db_contact.profile_cache = Some(cache.to_owned());
            }
        }

        Ok(db_contacts)
    }

    pub async fn fetch_one(
        pool: &SqlitePool,
        cache_pool: &SqlitePool,
        pubkey: &XOnlyPublicKey,
    ) -> Result<Option<DbContact>, Error> {
        let sql = format!("{} WHERE pubkey = ?", Self::FETCH_QUERY);

        let result = sqlx::query_as::<_, DbContact>(&sql)
            .bind(&pubkey.to_string())
            .fetch_optional(pool)
            .await?;

        if let Some(mut db_contact) = result {
            if let Some(cache) =
                ProfileCache::fetch_by_public_key(cache_pool, db_contact.pubkey()).await?
            {
                db_contact = db_contact.with_profile_cache(&cache);
            }

            Ok(Some(db_contact))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert_contact(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
        tracing::debug!("Upserting Contact {}", contact.pubkey().to_string());
        tracing::debug!("{:?}", contact);

        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        // SQL queries as static strings
        const UPDATE_SQL: &str = r#"
            UPDATE contact 
            SET relay_url=?, petname=?, updated_at=?
            WHERE pubkey=?
        "#;
        const INSERT_SQL: &str = r#"
            INSERT INTO contact 
                (pubkey, relay_url, petname, status, created_at, updated_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#;

        let mut tx = pool.begin().await?;

        // Try to update first
        let updated_rows = sqlx::query(UPDATE_SQL)
            .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&contact.petname)
            .bind(now_utc.timestamp_millis())
            .bind(&contact.pubkey.to_string())
            .execute(&mut tx)
            .await?
            .rows_affected();

        // If no rows were updated, insert the contact
        if updated_rows == 0 {
            sqlx::query(INSERT_SQL)
                .bind(&contact.pubkey.to_string())
                .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
                .bind(&contact.petname)
                .bind(contact.status as u8)
                .bind(contact.created_at.timestamp_millis())
                .bind(contact.updated_at.timestamp_millis())
                .execute(&mut tx)
                .await
                .map_err(|err| {
                    tracing::error!("Error upserting contact: {:?}", err);
                    err
                })?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
        tracing::info!("Updating Contact {}", contact.pubkey().to_string());
        tracing::debug!("{:?}", contact);
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = r#"
            UPDATE contact 
            SET relay_url=?, petname=?, status=?, updated_at=?
            WHERE pubkey=?
        "#;

        sqlx::query(sql)
            .bind(&contact.relay_url.as_ref().map(|url| url.to_string()))
            .bind(&contact.petname)
            .bind(contact.status as u8)
            .bind(now_utc.timestamp_millis())
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
        let sql = "DELETE FROM contact WHERE pubkey=?";

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub(crate) async fn have_contact(
        pool: &SqlitePool,
        pubkey: &XOnlyPublicKey,
    ) -> Result<bool, Error> {
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
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let pubkey = XOnlyPublicKey::from_str(&pubkey).map_err(|e| sqlx::Error::ColumnDecode {
            index: "pubkey".into(),
            source: Box::new(e),
        })?;
        let created_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("created_at")?, "created_at")?;
        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;

        let relay_url = row
            .get::<Option<String>, &str>("relay_url")
            .map(|url| unchecked_url_or_err(&url, "relay_url"))
            .transpose()?;

        let petname: Option<String> = row.get("petname");

        Ok(DbContact {
            profile_cache: None,
            pubkey,
            created_at,
            updated_at,
            petname,
            relay_url,
            status: row.get::<u8, &str>("status").into(),
        })
    }
}
