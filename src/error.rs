use nostr_sdk::{prelude::TagKind, EventId};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the nostrdome crate
#[derive(Error, Debug)]
pub enum Error {
    // General errors
    #[error("Unable to update contact: message ID is required but not provided.")]
    MissingMessageIdForContactUpdate,
    #[error("Can't update message without id")]
    MessageNotInDatabase,
    #[error("Can't update channel without id")]
    ChannelNotInDatabase,
    #[error("Event not in database: {0}")]
    EventNotInDatabase(EventId),
    #[error("No tags found in the event")]
    NoTags,
    #[error("Wrong tag type found")]
    WrongTag,
    #[error("Out-of-range number of seconds when parsing timestamp")]
    DbEventTimestampError,
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
    #[error("Invalid Unix timestamp: {0}")]
    InvalidTimestamp(i64),
    #[error("Database setup error: {0}")]
    DatabaseSetup(String),

    // FromDbEventError
    #[error("{0}")]
    FromDbEventError(#[from] FromDbEventError),

    // Formatting errors
    #[error("Formatting Error: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("Invalid URL: \"{0}\"")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Invalid URL Regex: \"{0}\"")]
    InvalidUrlRegex(String),
    #[error("Invalid URL Host: \"{0}\"")]
    InvalidUrlHost(String),
    #[error("Invalid URL Scheme: \"{0}\"")]
    InvalidUrlScheme(String),
    #[error("Parse integer error")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("JSON (de)serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Not a valid nostr relay url: {0}")]
    Url(String),

    // I/O Error
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    // Nostr Sdk errors
    #[error("Nostr Sdk Client Error: {0}")]
    NostrSdkError(#[from] nostr_sdk::client::Error),
    #[error("Nostr Sdk Key Error: {0}")]
    NostrSdkKeyError(#[from] nostr_sdk::key::Error),
    #[error("Nostr Sdk Event Builder Error: {0}")]
    NostrSdkEventBuilderError(#[from] nostr_sdk::prelude::builder::Error),
    #[error("Nostr secp256k1 Error: {0}")]
    NostrSecp256k1Error(#[from] nostr_sdk::secp256k1::Error),
    #[error("Invalid Date: {0}")]
    InvalidDate(String),
    #[error(
        "Database version is newer than supported by this executable (v{current} > v{db_ver})"
    )]
    NewerDbVersion { current: usize, db_ver: usize },
    #[error("TagKind is not P: {0}")]
    TagKindToContactError(TagKind),
    #[error("Other type of Tag")]
    TagToContactError,
    #[error("Not found contact with pubkey: {0}")]
    NotFoundContact(String),
    // Nostrdome specific errors
    #[error("Not allowed to insert own pubkey as a contact")]
    SameContactInsert,
    #[error("Not allowed to update to own pubkey as a contact")]
    SameContactUpdate,
    #[error("Write actions disabled for this relay: {0}")]
    WriteActionsDisabled(String),
    #[error("Not found any relay to write to")]
    NoRelayToWrite,

    // Channel-related error
    #[error("Failed to send message to a channel: {0}")]
    TrySendError(String),

    // Encryption and Decryption errors
    #[error("Decryption Error: {0}")]
    DecryptionError(String),
    #[error("Encryption Error: {0}")]
    EncryptionError(String),
    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),

    // UTF-8 error
    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

#[derive(Debug, Error)]
pub enum FromDbEventError {
    #[error("No tags found in the event")]
    NoTags,
    #[error("Wrong tag type found")]
    WrongTag,
    #[error("Event not in database: {0}")]
    EventNotInDatabase(EventId),
}
