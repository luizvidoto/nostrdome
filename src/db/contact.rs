use std::str::FromStr;

use nostr_sdk::{secp256k1::XOnlyPublicKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbContact {
    pub pubkey: XOnlyPublicKey,
    pub relay_url: Option<String>,
    pub petname: Option<String>,
    pub profile_image: Option<String>,
}

impl DbContact {
    pub fn new(pubkey: &XOnlyPublicKey) -> Self {
        Self {
            pubkey: pubkey.clone(),
            relay_url: None,
            petname: None,
            profile_image: None,
        }
    }
    pub fn from_str(pubkey: &str) -> Result<Self, Error> {
        let pubkey = XOnlyPublicKey::from_str(pubkey)?;
        Ok(Self::new(&pubkey))
    }
    pub fn from_tag(tag: &Tag) -> Result<Self, Error> {
        match tag {
            Tag::PubKey(pk, relay_url) => {
                Ok(Self::new(pk).with_relay_url(relay_url.to_owned().map(|url| url.to_string())))
            }
            Tag::ContactList {
                pk,
                relay_url,
                alias,
            } => Ok(Self::new(pk)
                .with_relay_url(relay_url.to_owned().map(|url| url.to_string()))
                .with_petname(alias)),
            _ => Err(Error::TagToContactError),
        }
    }
    fn with_relay_url(mut self, relay_url: Option<String>) -> Self {
        self.relay_url = relay_url;
        self
    }

    fn with_petname(mut self, petname: &Option<String>) -> Self {
        self.petname = petname.clone();
        self
    }
    pub fn relay_url(self, relay: &str) -> Self {
        if relay.is_empty() {
            self
        } else {
            Self {
                relay_url: Some(relay.to_owned()),
                ..self
            }
        }
    }
    pub fn petname(self, petname: &str) -> Self {
        if petname.is_empty() {
            self
        } else {
            Self {
                petname: Some(petname.to_owned()),
                ..self
            }
        }
    }
    pub fn profile_image(self, image: &str) -> Self {
        if image.is_empty() {
            self
        } else {
            Self {
                profile_image: Some(image.to_owned()),
                ..self
            }
        }
    }

    const FETCH_QUERY: &'static str =
        "SELECT pubkey, relay_url, petname, profile_image FROM contact";

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbContact>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let output = sqlx::query_as::<_, DbContact>(&sql).fetch_all(pool).await?;

        Ok(output)
    }

    pub async fn fetch_one(pool: &SqlitePool, pubkey: &str) -> Result<Option<DbContact>, Error> {
        let sql = format!("{} WHERE pubkey = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbContact>(&sql)
            .bind(pubkey)
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
        let sql = "INSERT OR IGNORE INTO contact (pubkey, relay_url, \
                   petname, profile_image) \
             VALUES (?1, ?2, ?3, ?4)";

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .bind(&contact.relay_url)
            .bind(&contact.petname)
            .bind(&contact.profile_image)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn insert_batch(pool: &SqlitePool, contacts: &[DbContact]) -> Result<(), Error> {
        let sql = "INSERT OR IGNORE INTO contact (pubkey, relay_url, \
               petname, profile_image) \
         VALUES (?1, ?2, ?3, ?4)";

        // Iniciar a transação
        let mut tx = pool.begin().await?;

        for contact in contacts {
            sqlx::query(sql)
                .bind(&contact.pubkey.to_string())
                .bind(&contact.relay_url)
                .bind(&contact.petname)
                .bind(&contact.profile_image)
                .execute(&mut tx)
                .await?;
        }

        // Fazer commit da transação
        tx.commit().await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
        let sql = "UPDATE contact SET relay_url=?, petname=?, profile_image=? WHERE pubkey=?";

        sqlx::query(sql)
            .bind(&contact.relay_url)
            .bind(&contact.petname)
            .bind(&contact.profile_image)
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, pubkey: &XOnlyPublicKey) -> Result<(), Error> {
        let sql = "DELETE FROM contact WHERE pubkey=?";

        sqlx::query(sql)
            .bind(&pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbContact {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let pubkey = row.try_get::<String, &str>("pubkey")?;
        Ok(DbContact {
            pubkey: XOnlyPublicKey::from_str(&pubkey).map_err(|e| sqlx::Error::ColumnDecode {
                index: "pubkey".into(),
                source: Box::new(e),
            })?,
            petname: row.try_get::<Option<String>, &str>("petname")?,
            relay_url: row.try_get::<Option<String>, &str>("relay_url")?,
            profile_image: row.try_get::<Option<String>, &str>("profile_image")?,
        })
    }
}
