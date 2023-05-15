use std::path::PathBuf;

use nostr_sdk::{prelude::TagKind, EventId};
use thiserror::Error;

use crate::{db::DbContactError, net::ImageSize};

// pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the nostrtalk crate
#[derive(Error, Debug)]
pub enum Error {
    // Image Errors
    #[error("Image error: {0}")]
    ImageError(#[from] image::error::ImageError),
    #[error("Invalid image extension: {0}")]
    ImageInvalidExtension(String),
    #[error("Invalid image format: {0}")]
    ImageInvalidFormat(String),
    #[error("Invalid image size: {0:?}")]
    InvalidImageSize(ImageSize),

    #[error("GITHUB_TOKEN not found")]
    GitHubTokenNotFound,
    #[error("Request error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Request error: {0}")]
    ReqwestStreamError(reqwest::Error),

    #[error("Request failed. Status code: {0}")]
    RequestFailed(reqwest::StatusCode),
    #[error("Tag name not found")]
    RequestTagNameNotFound,
    #[error("Not found any releases")]
    RequestReleaseNotFound,
    #[error("Failed to download image. Status code: {0}")]
    RequestImageError(reqwest::StatusCode),
    #[error("Missing content-type header")]
    RequestMissingContentType,

    #[error("Invalid content-type header: {0}")]
    ImageInvalidContentType(String),

    #[error("Error parsing JSON content into nostr_sdk::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Failed to send to nostr input channel: {0}")]
    FailedToSendNostrInput(String),

    // #[error("NTP Error: {0}")]
    // NtpError(#[from] ntp::errors::Error),
    #[error("Sntpc Error")]
    SntpcError,
    #[error("Sntpc Error: Unable to bind to any port")]
    NtpUnableToBindPort,
    #[error("Sntpc Error: Unable to set read timeout")]
    NtpUnableToSetReadTimeout,
    #[error("System time before unix epoch")]
    SystemTimeBeforeUnixEpoch,

    // General errors
    #[error("Unkown chat message: from_pubkey:{0} - to_pubkey:{1}")]
    UnknownChatMessage(String, String),
    #[error("Event need to be confirmed")]
    NotConfirmedEvent,
    #[error("Event is pending: {0}")]
    PendingEvent(EventId),
    #[error("Unable to update contact: message ID is required but not provided.")]
    MissingMessageIdForContactUpdate,
    #[error("Outdated contact insert")]
    OutDatedContactInsert,
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
    #[error("Invalid Unix timestamp: secs {0}, nanos {1}")]
    InvalidTimestampNanos(i64, u32),
    #[error("Database setup error: {0}")]
    DatabaseSetup(String),
    #[error("Not found project directory")]
    NotFoundProjectDirectory,

    #[error("Signing error: {0}")]
    SigningEventError(String),

    // FromDbContactError
    #[error("{0}")]
    FromDbContactError(#[from] DbContactError),

    // FromDbEventError
    #[error("{0}")]
    FromDbEventError(#[from] FromDbEventError),

    // Formatting errors
    #[error("Formatting Error: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("Invalid URL: \"{0}\"")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Invalid Relay URL: \"{0}\"")]
    InvalidRelayUrl(String),
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
    #[error("{0}")]
    ToStrError(#[from] reqwest::header::ToStrError),

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
    #[error("Nostr relay Error: {0}")]
    NostrSdkRelayError(#[from] nostr_sdk::relay::Error),
    #[error("Invalid Date: {0}")]
    InvalidDate(String),
    #[error(
        "Database version is newer than supported by this executable (v{current} > v{db_ver})"
    )]
    NewerDbVersion { current: usize, db_ver: usize },
    #[error("TagKind is not P: {0}")]
    TagKindToContactError(TagKind),

    // Nostrtalk specific errors
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

    #[error("Invalid path UTF-8 Error: {0}")]
    InvalidPath(PathBuf),
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
