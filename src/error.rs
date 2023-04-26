use nostr_sdk::{prelude::TagKind, EventId};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the nostrdome crate
#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to update contact: message ID is required but not provided.")]
    MissingMessageIdForContactUpdate,

    #[error("Can't update message without msg_id")]
    MessageNotInDatabase,

    #[error("{0}")]
    FromDbEventError(#[from] FromDbEventError),

    /// Event Update Error
    #[error("To update an event, insert first in the database: {0}")]
    EventNotInDatabase(EventId),

    /// Event Tags Empty
    #[error("Event tags field is empty")]
    NoTags,

    /// Event Tags Empty
    #[error("Wrong Tag")]
    WrongTag,

    /// DbEvent Error
    #[error("Out-of-range number of seconds when parsing timestamp")]
    DbEventTimestampError,

    /// Assertion failed
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    /// Formatting error
    #[error("Formatting Error: {0}")]
    Fmt(#[from] std::fmt::Error),

    /// Formatting error
    #[error("Invalid Unix timestamp: {0}")]
    InvalidTimestamp(i64),

    /// Hex string decoding error
    // #[error("Hex Decode Error: {0}")]
    // HexDecode(#[from] hex::FromHexError),

    #[error("Database setup error: {0}")]
    DatabaseSetup(String),

    /// I/O Error
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid URL
    #[error("Invalid URL: \"{0}\"")]
    InvalidUrl(#[from] url::ParseError),

    /// Invalid URL
    #[error("Invalid URL Regex: \"{0}\"")]
    InvalidUrlRegex(String),

    /// Invalid URL Host
    #[error("Invalid URL Host: \"{0}\"")]
    InvalidUrlHost(String),

    /// Invalid URL Scheme
    #[error("Invalid URL Scheme: \"{0}\"")]
    InvalidUrlScheme(String),

    /// Parse integer error
    #[error("Parse integer error")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Serialization error
    #[error("JSON (de)serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// Sqlx Error
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    /// Url Error
    #[error("Not a valid nostr relay url: {0}")]
    Url(String),

    /// UTF-8 error
    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// Nostr Sdk Error
    #[error("Nostr Sdk Client Error: {0}")]
    NostrSdkError(#[from] nostr_sdk::client::Error),

    /// Nostr Sdk Keys Error
    #[error("Nostr Sdk Key Error: {0}")]
    NostrSdkKeyError(#[from] nostr_sdk::key::Error),

    /// Nostr Sdk Event Builder Error
    #[error("Nostr Sdk Event Builder Error: {0}")]
    NostrSdkEventBuilderError(#[from] nostr_sdk::prelude::builder::Error),

    /// Nostr Sdk secp256k1 Error
    #[error("Nostr secp256k1 Error: {0}")]
    NostrSecp256k1Error(#[from] nostr_sdk::secp256k1::Error),

    /// Invalid Date
    #[error("Invalid Date: {0}")]
    InvalidDate(String),

    /// Database versione newer than supported
    #[error(
        "Database version is newer than supported by this executable (v{current} > v{db_ver})"
    )]
    NewerDbVersion { current: usize, db_ver: usize },

    /// Error when converting nostr_sdk::Tag into DbContact. TagKind is not P.
    #[error("TagKind is not P: {0}")]
    TagKindToContactError(TagKind),

    /// Error when converting nostr_sdk::Tag into DbContact. Other type of Tag.
    #[error("Other type of Tag")]
    TagToContactError,

    /// Error when contact not found with pubkey
    #[error("Not found contact with pubkey: {0}")]
    NotFoundContact(String),

    /// Error when user tries to insert own pubkey as a new contact
    #[error("Not allowed to insert own pubkey as a contact")]
    SameContactInsert,

    /// Error when user tries to insert own pubkey as a new contact
    #[error("Write actions disabled for this realy: {0}")]
    WriteActionsDisabled(String),

    /// Error when user tries to emit an event but no relay is writable
    #[error("Not found any relay to write to")]
    NoRelayToWrite,

    /// Error occured when at Decryption
    #[error("Decryption Error: {0}")]
    DecryptionError(String),

    /// Error occured when at Encryption
    #[error("Encryption Error: {0}")]
    EncryptionError(String),

    /// Error occured in the AES GCM crate
    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),
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
