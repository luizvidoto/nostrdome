use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::db::DbEvent;

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
        self.created_at.as_i64()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message created at using unix timestamp
    pub created_at: i64,
    /// Message content
    pub content: String,
    /// Pub key of the author of the message
    pub from_pubkey: XOnlyPublicKey,
}

impl ChatMessage {
    pub fn from_event<S, E>(event: &E, decrypted_message: S) -> Self
    where
        S: Into<String>,
        E: EventLike,
    {
        Self {
            content: decrypted_message.into(),
            created_at: event.created_at(),
            from_pubkey: event.pubkey(),
        }
    }
}
