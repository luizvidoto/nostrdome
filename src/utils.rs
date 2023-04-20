use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use chrono::NaiveDateTime;
use nostr_sdk::prelude::*;
use serde::de::DeserializeOwned;

use crate::error::Error;

// Accepts both hex and bech32 keys and returns the hex encoded key
pub fn parse_key(key: String) -> Result<String, anyhow::Error> {
    // Check if the key is a bech32 encoded key
    let parsed_key = if key.starts_with("npub") {
        XOnlyPublicKey::from_bech32(key)?.to_string()
    } else if key.starts_with("nsec") {
        SecretKey::from_bech32(key)?.display_secret().to_string()
    } else if key.starts_with("note") {
        EventId::from_bech32(key)?.to_hex()
    } else if key.starts_with("nchannel") {
        ChannelId::from_bech32(key)?.to_hex()
    } else {
        // If the key is not bech32 encoded, return it as is
        key
    };
    Ok(parsed_key)
}

pub fn json_reader<P, T: DeserializeOwned>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
{
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let content = serde_json::from_reader(buf_reader)?;
    Ok(content)
}

pub fn json_to_string<P>(path: P) -> Result<String, io::Error>
where
    P: AsRef<Path>,
{
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn format_pubkey(pubkey: &str) -> String {
    let prefix = &pubkey[0..4];
    let suffix = &pubkey[pubkey.len().saturating_sub(4)..];
    format!("{}..{}", prefix, suffix)
}

/// Convert a i64 representing milliseconds since UNIX epoch to an Option<NaiveDateTime>.
///
/// # Arguments
///
/// * `millis` - A i64 representing the number of milliseconds since the UNIX epoch (1970-01-01 00:00:00 UTC)
///
/// # Returns
///
/// * Am Option with a NaiveDateTime representing the given milliseconds since UNIX epoch
pub fn millis_to_naive(millis: i64) -> Option<NaiveDateTime> {
    // Calculate the seconds and nanoseconds components of the input timestamp
    let ts_secs = millis / 1000;
    let ts_ns = (millis % 1000) * 1_000_000;

    // Convert the seconds and nanoseconds to a NaiveDateTime
    NaiveDateTime::from_timestamp_opt(ts_secs, ts_ns as u32)
}

pub fn event_tt_to_naive(timestamp: nostr_sdk::Timestamp) -> Option<NaiveDateTime> {
    let as_milli = timestamp.as_i64() * 1000;
    millis_to_naive(as_milli)
}

pub fn handle_decode_error<E>(error: E, index: &str) -> sqlx::Error
where
    E: std::error::Error + 'static + Send + Sync,
{
    sqlx::Error::ColumnDecode {
        index: index.into(),
        source: Box::new(error),
    }
}
