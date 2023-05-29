use futures::channel::mpsc;
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use thiserror::Error;

// pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the nostrtalk crate

#[derive(Error, Debug)]
pub enum Error {
    #[error("SendError: {0}")]
    FromSendError(#[from] mpsc::SendError),

    // I/O Error
    #[error("I/O Error: {0}")]
    FromIo(#[from] std::io::Error),

    #[error("JSON (de)serialization error: {0}")]
    FromSerdeJson(#[from] serde_json::Error),

    #[error("Reqwest error: {0}")]
    FromReqwestClientError(#[from] crate::net::reqwest_client::Error),

    #[error("Nostr Client Error: {0}")]
    FromNostrClientError(#[from] ns_client::Error),

    #[error("Nostr Client Sdk Error: {0}")]
    FromNostrClientSdkError(#[from] nostr_sdk::client::Error),

    #[error("{0}")]
    FromChatMessageError(#[from] crate::types::chat_message::Error),

    #[error("{0}")]
    FromChannelCacheError(#[from] crate::db::channel_cache::Error),

    #[error("{0}")]
    FromContactError(#[from] crate::db::contact::Error),

    #[error("{0}")]
    FromDatabaseError(#[from] crate::db::database::Error),

    #[error("{0}")]
    FromEventError(#[from] crate::db::event::Error),

    #[error("{0}")]
    FromMessageError(#[from] crate::db::message::Error),

    #[error("{0}")]
    FromProfileCacheError(#[from] crate::db::profile_cache::Error),

    #[error("{0}")]
    FromRelayError(#[from] crate::db::relay::Error),

    #[error("{0}")]
    FromRelayResponseError(#[from] crate::db::relay_response::Error),

    #[error("{0}")]
    FromUserConfigError(#[from] crate::db::user_config::Error),

    #[error("{0}")]
    FromEventBuilderError(#[from] crate::net::operations::builder::Error),

    #[error("{0}")]
    FromNtpError(#[from] crate::net::ntp::Error),

    #[error("Failed to send to backend input channel: {0}")]
    FailedToSendBackendInput(String),

    #[error("App didn't ask for kind: {0:?}")]
    NotSubscribedToKind(nostr::Kind),

    #[error("Contact not found - pubkey: {0:?}")]
    ContactNotFound(XOnlyPublicKey),

    #[error("Not allowed to insert own pubkey as a contact")]
    SameContactInsert,

    #[error("Not allowed to update to own pubkey as a contact")]
    SameContactUpdate,

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(nostr::EventId),

    #[error("Trying to find message with event_id: {0}")]
    EventWithoutMessage(i64),

    #[error("Inserting event with the same id")]
    DuplicatedEvent,

    #[error("Event not in database: {0}")]
    EventNotInDatabase(nostr::EventId),

    #[error("Failed to insert event")]
    FailedToInsert,
}
