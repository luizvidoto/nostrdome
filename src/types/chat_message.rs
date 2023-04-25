use chrono::NaiveDateTime;
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::{
    db::{DbContact, DbEvent, DbMessage},
    utils::millis_to_naive,
};

pub trait EventLike {
    fn created_at(&self) -> i64;
    fn pubkey(&self) -> XOnlyPublicKey;
}

impl EventLike for nostr_sdk::Event {
    fn created_at(&self) -> i64 {
        self.created_at.as_i64()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

impl EventLike for DbEvent {
    fn created_at(&self) -> i64 {
        self.created_at.timestamp_millis()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message created at using unix timestamp
    pub created_at: Option<NaiveDateTime>,
    /// Decrypted message content
    pub content: String,
    /// Pub key of the author of the message
    pub from_pubkey: XOnlyPublicKey,
    pub is_from_user: bool,
    pub petname: Option<String>,
}

impl ChatMessage {
    pub fn from_event<S, E>(
        event: &E,
        decrypted_message: S,
        is_from_user: bool,
        contact: &DbContact,
    ) -> Self
    where
        S: Into<String>,
        E: EventLike,
    {
        Self {
            content: decrypted_message.into(),
            created_at: millis_to_naive(event.created_at()),
            from_pubkey: event.pubkey(),
            // is_from_user: &event.pubkey() == user_pubkey,
            is_from_user,
            petname: contact.petname.clone(),
        }
    }
    pub fn from_db_message(
        db_message: &DbMessage,
        is_from_user: bool,
        contact: &DbContact,
        content: &str,
    ) -> Self {
        Self {
            content: content.to_owned(),
            created_at: Some(db_message.created_at),
            from_pubkey: db_message.from_pubkey.clone(),
            is_from_user,
            petname: contact.petname.clone(),
        }
    }
}
