use std::result::Result as StdResult;
use std::str::FromStr;

use chrono::{NaiveDateTime, Utc};
use nostr_sdk::{secp256k1::XOnlyPublicKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::{Error, Result},
    types::ChatMessage,
    utils::{millis_to_naive, millis_to_naive_or_err},
};

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
    relay_url: Option<String>,
    petname: Option<String>,
    profile_image: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    status: ContactStatus,
    unseen_messages: u8,
    last_message_content: Option<String>,
    last_message_date: Option<NaiveDateTime>,
}

impl PartialEq for DbContact {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}

impl DbContact {
    // const FETCH_QUERY: &'static str = r#"
    //     SELECT contact.pubkey, contact.petname, contact.relay_url, contact.profile_image, contact.status,
    //         contact.created_at AS contact_created_at, contact.updated_at, contact.unseen_messages,
    //         message.created_at AS message_created_at, message.content, message.from_pubkey, message.msg_id
    //     FROM contact
    //     LEFT JOIN message ON contact.last_message_id = message.msg_id
    // "#;
    const FETCH_QUERY: &'static str = r#"SELECT * FROM contact"#;

    pub fn new(pubkey: &XOnlyPublicKey) -> Self {
        Self {
            pubkey: pubkey.clone(),
            relay_url: None,
            petname: None,
            profile_image: None,
            status: ContactStatus::Unknown,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            unseen_messages: 0,
            last_message_content: None,
            last_message_date: None,
        }
    }
    // pub fn from_temp_contact(
    //     temp_contact: &TempContact,
    //     last_message: Option<ChatMessage>,
    // ) -> Self {
    //     Self {
    //         pubkey: temp_contact.ct_pubkey,
    //         relay_url: temp_contact.ct_relay_url.to_owned(),
    //         petname: temp_contact.ct_petname.to_owned(),
    //         profile_image: temp_contact.ct_profile_image.to_owned(),
    //         status: temp_contact.ct_status,
    //         unseen_messages: temp_contact.ct_unseen_messages,
    //         created_at: temp_contact.ct_created_at,
    //         updated_at: temp_contact.ct_updated_at,
    //         last_message,
    //     }
    // }

    pub fn from_tag(tag: &Tag) -> Result<Self> {
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

    pub fn pubkey(&self) -> &XOnlyPublicKey {
        &self.pubkey
    }
    pub fn from_str(pubkey: &str) -> Result<Self> {
        let pubkey = XOnlyPublicKey::from_str(pubkey)?;
        Ok(Self::new(&pubkey))
    }
    pub fn get_petname(&self) -> Option<String> {
        self.petname.clone()
    }
    pub fn get_relay_url(&self) -> Option<String> {
        self.relay_url.clone()
    }
    pub fn get_profile_image(&self) -> Option<String> {
        self.profile_image.clone()
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

    pub async fn new_message(
        pool: &SqlitePool,
        db_contact: &DbContact,
        chat_message: &ChatMessage,
    ) -> Result<DbContact> {
        // do not update unseen count here because we may be in the chat
        if Some(&chat_message.created_at) >= db_contact.last_message_date.as_ref() {
            let sql = r#"
                UPDATE contact 
                SET updated_at=?, last_message_content=?, last_message_date=?
                WHERE pubkey=?
            "#;

            sqlx::query(sql)
                .bind(Utc::now().timestamp_millis())
                .bind(&chat_message.content)
                .bind(chat_message.created_at.timestamp_millis())
                .bind(&db_contact.pubkey.to_string())
                .execute(pool)
                .await?;
        } else {
            tracing::info!("Can't update last_message with an older message.");
        }

        let db_contact_updated = Self::fetch_one(pool, &db_contact.pubkey)
            .await?
            .ok_or(Error::NotFoundContact(db_contact.pubkey().to_string()))?;

        Ok(db_contact_updated)
    }

    pub async fn add_to_unseen_count(
        pool: &SqlitePool,
        db_contact: &DbContact,
    ) -> Result<DbContact> {
        let sql = r#"
                UPDATE contact
                SET updated_at = ?, unseen_messages = unseen_messages + 1
                WHERE pubkey = ?
            "#;

        sqlx::query(sql)
            .bind(Utc::now().timestamp_millis())
            .bind(&db_contact.pubkey.to_string())
            .execute(pool)
            .await?;

        let db_contact_updated = Self::fetch_one(pool, &db_contact.pubkey)
            .await?
            .ok_or(Error::NotFoundContact(db_contact.pubkey().to_string()))?;

        Ok(db_contact_updated)
    }

    pub async fn update_unseen_count(
        pool: &SqlitePool,
        db_contact: &DbContact,
        count: u8,
    ) -> Result<DbContact> {
        tracing::info!("updated contact count: {}", count);
        let sql = r#"
                UPDATE contact 
                SET updated_at=?, unseen_messages=?
                WHERE pubkey=?
            "#;

        sqlx::query(sql)
            .bind(Utc::now().timestamp_millis())
            .bind(count)
            .bind(&db_contact.pubkey.to_string())
            .execute(pool)
            .await?;

        let db_contact_updated = Self::fetch_one(pool, &db_contact.pubkey)
            .await?
            .ok_or(Error::NotFoundContact(db_contact.pubkey().to_string()))?;

        Ok(db_contact_updated)
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
        let sql = r#"
            INSERT INTO contact 
                (pubkey, relay_url, petname, profile_image, status, 
                    unseen_messages, created_at, updated_at, last_message_content, last_message_date) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .bind(&contact.relay_url)
            .bind(&contact.petname)
            .bind(&contact.profile_image)
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
            .execute(tx)
            .await?;

        Ok(())
    }

    pub async fn insert(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        tracing::info!("Inserting Contact: {:?}", contact);

        // Iniciar a transação
        let mut tx = pool.begin().await?;

        // Chamar a função auxiliar para inserir o contato
        Self::insert_single_contact(&mut tx, contact).await?;

        // Fazer commit da transação
        tx.commit().await?;

        Ok(())
    }

    pub async fn insert_batch(pool: &SqlitePool, contacts: &[DbContact]) -> Result<()> {
        tracing::info!("Inserting Batch of contacts");

        // Iniciar a transação
        let mut tx = pool.begin().await?;

        for contact in contacts {
            // Chamar a função auxiliar para inserir o contato
            Self::insert_single_contact(&mut tx, contact).await?;
        }

        // Fazer commit da transação
        tx.commit().await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        let sql = r#"
            UPDATE contact 
            SET relay_url=?, petname=?, profile_image=?, 
                status=?, unseen_messages=?,  
                last_message_content=?, last_message_date=?,
                updated_at=?
            WHERE pubkey=?
        "#;

        sqlx::query(sql)
            .bind(&contact.relay_url)
            .bind(&contact.petname)
            .bind(&contact.profile_image)
            .bind(contact.status as u8)
            .bind(contact.unseen_messages)
            .bind(&contact.last_message_content)
            .bind(
                contact
                    .last_message_date
                    .as_ref()
                    .map(|date| date.timestamp_millis()),
            )
            .bind(Utc::now().timestamp_millis())
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    // pub async fn update_last_message(pool: &SqlitePool, contact: &DbContact) -> Result<(), Error> {
    //     let last_msg = contact
    //         .last_message
    //         .as_ref()
    //         .ok_or(Error::MissingMessageIdForContactUpdate)?;
    //     let sql = "UPDATE contact SET updated_at=?, last_message_id=? WHERE pubkey=?";

    //     sqlx::query(sql)
    //         .bind(Utc::now().timestamp_millis())
    //         .bind(last_msg.msg_id)
    //         .bind(&contact.pubkey.to_string())
    //         .execute(pool)
    //         .await?;

    //     Ok(())
    // }

    pub async fn delete(pool: &SqlitePool, contact: &DbContact) -> Result<()> {
        let sql = "DELETE FROM contact WHERE pubkey=?";

        sqlx::query(sql)
            .bind(&contact.pubkey.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }
}

// fn temp_contact_to_db_contact(row: &TempContact) -> Result<DbContact, Error> {
//     let last_message = match (
//         row.msg_id,
//         row.msg_created_at,
//         &row.msg_content,
//         &row.msg_from_pubkey,
//     ) {
//         (Some(msg_id), Some(created_at), Some(content), Some(from_pubkey)) => Some(ChatMessage {
//             msg_id,
//             created_at: millis_to_naive(created_at),
//             content: content.to_owned(),
//             from_pubkey: XOnlyPublicKey::from_str(from_pubkey)?,
//             is_from_user: false, // não importa
//             petname: row.ct_petname.clone(),
//         }),
//         _ => None,
//     };
//     Ok(DbContact::from_temp_contact(row, last_message))
// }

impl sqlx::FromRow<'_, SqliteRow> for DbContact {
    fn from_row(row: &'_ SqliteRow) -> StdResult<Self, sqlx::Error> {
        let pubkey = row.try_get::<String, &str>("pubkey")?;
        let created_at = millis_to_naive_or_err(
            row.try_get::<i64, &str>("created_at")?,
            "db_contact created_at",
        )?;
        let updated_at = millis_to_naive_or_err(
            row.try_get::<i64, &str>("updated_at")?,
            "db_contact updated_at",
        )?;

        Ok(DbContact {
            pubkey: XOnlyPublicKey::from_str(&pubkey).map_err(|e| sqlx::Error::ColumnDecode {
                index: "pubkey".into(),
                source: Box::new(e),
            })?,
            created_at,
            updated_at,
            petname: row.try_get::<Option<String>, &str>("petname")?,
            relay_url: row.try_get::<Option<String>, &str>("relay_url")?,
            profile_image: row.try_get::<Option<String>, &str>("profile_image")?,
            status: row.get::<u8, &str>("status").into(),
            unseen_messages: row.try_get::<i64, &str>("unseen_messages")? as u8,
            last_message_content: row.get::<Option<String>, &str>("last_message_content"),
            last_message_date: row
                .get::<Option<i64>, &str>("last_message_date")
                .map(|n| millis_to_naive(n)),
        })
    }
}

// #[derive(Debug, Deserialize)]
// pub struct TempContact {
//     pub ct_pubkey: XOnlyPublicKey,
//     pub ct_relay_url: Option<String>,
//     pub ct_petname: Option<String>,
//     pub ct_profile_image: Option<String>,
//     pub ct_status: ContactStatus,
//     pub ct_unseen_messages: u8,
//     pub ct_created_at: NaiveDateTime,
//     pub ct_updated_at: NaiveDateTime,
//     pub msg_id: Option<i64>,
//     pub msg_created_at: Option<i64>,
//     pub msg_content: Option<String>,
//     pub msg_from_pubkey: Option<String>,
// }

// impl sqlx::FromRow<'_, SqliteRow> for TempContact {
//     fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
//         let ct_created_at = millis_to_naive_or_err(
//             row.try_get::<i64, &str>("contact_created_at")?,
//             "temp contact_created_at",
//         )?;
//         let ct_updated_at =
//             millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "temp updated_at")?;
//         let pubkey = row.try_get::<String, &str>("pubkey")?;
//         Ok(TempContact {
//             ct_pubkey: XOnlyPublicKey::from_str(&pubkey).map_err(|e| {
//                 sqlx::Error::ColumnDecode {
//                     index: "pubkey".into(),
//                     source: Box::new(e),
//                 }
//             })?,
//             ct_created_at,
//             ct_updated_at,
//             ct_petname: row.try_get::<Option<String>, &str>("petname")?,
//             ct_relay_url: row.try_get::<Option<String>, &str>("relay_url")?,
//             ct_profile_image: row.try_get::<Option<String>, &str>("profile_image")?,
//             ct_status: row.get::<u8, &str>("status").into(),
//             ct_unseen_messages: row.try_get::<i64, &str>("unseen_messages")? as u8,
//             msg_id: row.get::<Option<i64>, &str>("msg_id"),
//             msg_created_at: row.get::<Option<i64>, &str>("message_created_at"),
//             msg_from_pubkey: row.get::<Option<String>, &str>("from_pubkey"),
//             msg_content: row.get::<Option<String>, &str>("content"),
//         })
//     }
// }
