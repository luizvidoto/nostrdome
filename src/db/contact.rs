use chrono::{NaiveDateTime, Utc};
use iced::widget::image::Handle;
use nostr::prelude::{FromBech32, ToBech32};
use nostr::EventId;
use nostr::{secp256k1::XOnlyPublicKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::str::FromStr;
use thiserror::Error;
use url::Url;

use crate::consts::default_profile_image;
use crate::db::UserConfig;
use crate::error::BackendClosed;
use crate::net::{self, BackEndConnection, ImageKind, ImageSize};
use crate::utils::millis_to_naive_or_err;
use crate::utils::url_or_err;

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
    TagToContact,

    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("{0}")]
    FromProfileCache(#[from] crate::db::profile_cache::Error),

    #[error("Error parsing url: {0}")]
    FromUrlParse(#[from] url::ParseError),
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
    relay_url: Option<Url>,
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
            relay_url: c.relay_url.as_ref().map(|url| url.to_string().into()),
            alias: c.petname.to_owned(),
        }
    }
}

// impl From<&nostr::Contact> for DbContact {
//     fn from(c: &nostr::Contact) -> Self {
//         let relay_url = c
//             .relay_url
//             .as_ref()
//             .and_then(|url| Url::parse(&url.to_string()).ok());

//         Self {
//             created_at: Utc::now().naive_utc(),
//             updated_at: Utc::now().naive_utc(),
//             relay_url,
//             petname: c.alias.to_owned(),
//             profile_cache: None,
//             pubkey: c.pk.to_owned(),
//             status: ContactStatus::Unknown,
//         }
//     }
// }
// impl From<nostr::Contact> for DbContact {
//     fn from(c: nostr::Contact) -> Self {
//         c.into()
//     }
// }

impl PartialEq for DbContact {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}

impl DbContact {
    const FETCH_QUERY: &'static str = r#"SELECT * FROM contact"#;

    pub fn new(pubkey: &XOnlyPublicKey) -> Self {
        Self {
            pubkey: *pubkey,
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
                    contact = contact.with_relay_url(&relay_url.to_string());
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
                    contact = contact.with_relay_url(&relay_url.to_string());
                }

                if let Some(petname) = alias {
                    contact = contact.with_petname(petname);
                }

                Ok(contact)
            }
            _ => Err(Error::TagToContact),
        }
    }

    pub fn pubkey(&self) -> &XOnlyPublicKey {
        &self.pubkey
    }

    pub fn from_pubkey(pubkey: &str) -> Result<Self, Error> {
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
        let db_contact = Self::from_pubkey(pubkey)?;
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
    pub fn get_profile_event_hash(&self) -> Option<EventId> {
        self.profile_cache
            .as_ref()
            .map(|profile| profile.event_hash)
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
    pub fn get_relay_url(&self) -> Option<Url> {
        self.relay_url.clone()
    }
    pub fn with_profile_cache(mut self, cache: &ProfileCache) -> Self {
        self.profile_cache = Some(cache.clone());
        self
    }
    pub fn with_relay_url(mut self, relay_url: &str) -> Self {
        let url = Url::parse(relay_url).ok();
        self.relay_url = url;
        self
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

        self.pubkey().to_bech32().unwrap_or(self.pubkey.to_string())
    }

    fn update_relay_url(&mut self, relay_url: &str) -> Result<(), Error> {
        let url = Url::parse(relay_url)?;
        self.relay_url = Some(url);
        Ok(())
    }

    pub fn profile_image(
        &self,
        size: ImageSize,
        conn: &mut BackEndConnection,
    ) -> Result<Handle, BackendClosed> {
        if let Some(cache) = &self.profile_cache {
            let kind = ImageKind::Profile;

            if let Some(img_cache) = &cache.profile_pic_cache {
                let path = img_cache.sized_image(size);
                return Ok(Handle::from_path(path));
            } else if let Some(image_url) = &cache.metadata.picture {
                conn.send(net::ToBackend::DownloadImage {
                    image_url: image_url.to_owned(),
                    kind,
                    identifier: self.pubkey.to_string(),
                    event_hash: cache.event_hash.to_owned(),
                })?;
            } else {
                tracing::debug!("Contact don't have profile image");
            }
        } else {
            tracing::debug!("no profile cache for contact: {}", self.pubkey.to_string());
        }

        Ok(Handle::from_memory(default_profile_image(size)))
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

    pub async fn insert(pool: &SqlitePool, pubkey: &XOnlyPublicKey) -> Result<i64, Error> {
        let utc_now = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let output =
            sqlx::query("INSERT INTO contact (pubkey, created_at, updated_at) VALUES (?, ?, ?);")
                .bind(&pubkey.to_string())
                .bind(utc_now.timestamp_millis())
                .bind(utc_now.timestamp_millis())
                .execute(pool)
                .await?;

        Ok(output.last_insert_rowid())
    }

    pub async fn fetch_insert(
        pool: &SqlitePool,
        cache_pool: &SqlitePool,
        pubkey: &XOnlyPublicKey,
    ) -> Result<DbContact, Error> {
        let sql = format!("{} WHERE pubkey = ?", Self::FETCH_QUERY);

        let result = sqlx::query_as::<_, DbContact>(&sql)
            .bind(&pubkey.to_string())
            .fetch_optional(pool)
            .await?;

        let mut db_contact = if result.is_none() {
            let last_insert_rowid = Self::insert(pool, pubkey).await?;
            let sql = format!("{} WHERE id = ?", Self::FETCH_QUERY);
            sqlx::query_as::<_, DbContact>(&sql)
                .bind(last_insert_rowid)
                .fetch_one(pool)
                .await?
        } else {
            result.unwrap()
        };

        if let Some(cache) =
            ProfileCache::fetch_by_public_key(cache_pool, db_contact.pubkey()).await?
        {
            db_contact = db_contact.with_profile_cache(&cache);
        }

        Ok(db_contact)
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

        let utc_now = UserConfig::get_corrected_time(pool)
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
            .bind(utc_now.timestamp_millis())
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
        let utc_now = UserConfig::get_corrected_time(pool)
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
            .bind(utc_now.timestamp_millis())
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
    pub async fn delete_all(pool: &SqlitePool) -> Result<(), Error> {
        let sql = "DELETE FROM contact;";

        sqlx::query(sql).execute(pool).await?;

        Ok(())
    }
    pub async fn has_contact(pool: &SqlitePool, pubkey: &XOnlyPublicKey) -> Result<bool, Error> {
        let sql = "SELECT EXISTS(SELECT 1 FROM contact WHERE pubkey=?)";

        let exists: (bool,) = sqlx::query_as(sql)
            .bind(pubkey.to_string())
            .fetch_one(pool)
            .await?;

        Ok(exists.0)
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
            .filter(|url| !url.is_empty())
            .map(|url| url_or_err(&url, "relay_url"))
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
