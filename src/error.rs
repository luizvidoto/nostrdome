use thiserror::Error;

/// Errors that can occur in the nostrdome crate
#[derive(Error, Debug)]
pub enum Error {
    /// Assertion failed
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    /// Formatting error
    #[error("Formatting Error: {0}")]
    Fmt(#[from] std::fmt::Error),

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
    #[error("{0}")]
    NostrSdkError(#[from] NostrSdkError),

    /// Invalid Date
    #[error("Invalid Date: {0}")]
    InvalidDate(String),

    /// Database versione newer than supported
    #[error(
        "Database version is newer than supported by this executable (v{current} > v{db_ver})"
    )]
    NewerDbVersion { current: usize, db_ver: usize },
}

/// Errors that can occur in the nostr-sdk crate
#[derive(Error, Debug)]
pub enum NostrSdkError {
    /// Error converting from hex string to EventId
    #[error("HexToEventIdError: {0}")]
    HexToEventIdError(String),

    /// Bad public key
    #[error("Invalid Public Key")]
    InvalidPublicKey,
}
