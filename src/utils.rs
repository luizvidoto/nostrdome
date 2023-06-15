#![allow(dead_code)]
use crate::{
    components::chat_contact::ChatContact,
    db::{DbContact, MessageStatus},
    net::ImageKind,
    style::{Theme, ThemeType},
    types::ChannelMetadata,
};
use chrono::{DateTime, Local, NaiveDateTime, Offset};
use iced::widget::image::Handle;
use image::{ImageBuffer, Luma, Rgba};
use nostr::prelude::*;
use qrcode::QrCode;
use regex::Regex;
use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
    str::FromStr,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Nostr Nip 19 Error: {0}")]
    ParseNip19Error(#[from] nostr::nips::nip19::Error),

    #[error("JSON (de)serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid Unix timestamp: {0}")]
    InvalidTimestamp(i64),

    #[error("Invalid Unix timestamp: secs {0}, nanos {1}")]
    InvalidTimestampNanos(i64, u32),

    #[error("{0}")]
    QrError(#[from] qrcode::types::QrError),

    #[error("{0}")]
    FromRegexError(#[from] regex::Error),

    #[error("{0}")]
    FromParseError(#[from] std::num::ParseIntError),
}

// Accepts both hex and bech32 keys and returns the hex encoded key
pub fn parse_key(key: String) -> Result<String, Error> {
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

/// Convert a i64 representing milliseconds since UNIX epoch to an Option<NaiveDateTime>.
///
/// # Arguments
///
/// * `millis` - A i64 representing the number of milliseconds since the UNIX epoch (1970-01-01 00:00:00 UTC)
///
/// # Returns
///
/// * Am Option with a NaiveDateTime representing the given milliseconds since UNIX epoch
pub fn millis_to_naive(millis: i64) -> Result<NaiveDateTime, Error> {
    // Calculate the seconds and nanoseconds components of the input timestamp
    let ts_secs = millis / 1000;
    let ts_ns = (millis % 1000) * 1_000_000;

    // Convert the seconds and nanoseconds to a NaiveDateTime
    NaiveDateTime::from_timestamp_opt(ts_secs, ts_ns as u32).ok_or(Error::InvalidTimestamp(millis))
}

pub fn ns_event_to_millis(created_at: nostr::Timestamp) -> i64 {
    created_at.as_i64() * 1000
}

pub fn ns_event_to_naive(created_at: nostr::Timestamp) -> Result<NaiveDateTime, Error> {
    Ok(millis_to_naive(ns_event_to_millis(created_at))?)
}

pub fn naive_to_event_tt(naive_utc: NaiveDateTime) -> nostr::Timestamp {
    nostr::Timestamp::from(naive_utc.timestamp() as u64)
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

pub fn millis_to_naive_or_err(millis: i64, index: &str) -> Result<NaiveDateTime, sqlx::Error> {
    Ok(
        millis_to_naive(millis).map_err(|e| sqlx::Error::ColumnDecode {
            index: index.into(),
            source: Box::new(e),
        })?,
    )
}
pub fn public_key_or_err(public_key: &str, index: &str) -> Result<XOnlyPublicKey, sqlx::Error> {
    XOnlyPublicKey::from_str(public_key).map_err(|e| handle_decode_error(e, index))
}
pub fn event_hash_or_err(event_id: &str, index: &str) -> Result<EventId, sqlx::Error> {
    EventId::from_str(event_id).map_err(|e| handle_decode_error(e, index))
}
pub fn url_or_err(url: &str, index: &str) -> Result<Url, sqlx::Error> {
    Url::parse(url).map_err(|e| handle_decode_error(e, index))
}
pub fn profile_meta_or_err(json: &str, index: &str) -> Result<nostr::Metadata, sqlx::Error> {
    nostr::Metadata::from_json(json).map_err(|e| handle_decode_error(e, index))
}
pub fn channel_meta_or_err(json: &str, index: &str) -> Result<ChannelMetadata, sqlx::Error> {
    ChannelMetadata::from_json(json).map_err(|e| handle_decode_error(e, index))
}
pub fn image_kind_or_err(kind: i32, index: &str) -> Result<ImageKind, sqlx::Error> {
    ImageKind::from_i32(kind).map_err(|e| handle_decode_error(e, index))
}
pub fn relay_doc_or_err(doc: &str, index: &str) -> Result<RelayInformationDocument, sqlx::Error> {
    serde_json::from_str(&doc).map_err(|e| handle_decode_error(e, index))
}
pub fn message_status_or_err(status: i32, index: &str) -> Result<MessageStatus, sqlx::Error> {
    MessageStatus::from_i32(status).map_err(|e| handle_decode_error(e, index))
}
pub fn theme_or_err(theme: u8, index: &str) -> Result<Theme, sqlx::Error> {
    u8::try_into(theme).map_err(|e| handle_decode_error(e, index))
}

pub fn chat_matches_search(chat: &ChatContact, search: &str) -> bool {
    let selected_name = chat.contact.select_name();
    selected_name
        .to_lowercase()
        .contains(&search.to_lowercase())
}

pub fn url_matches_search(url: &Url, search: &str) -> bool {
    url.as_str().to_lowercase().contains(&search.to_lowercase())
}

pub fn from_naive_utc_to_local(naive_utc: NaiveDateTime) -> DateTime<Local> {
    DateTime::from_utc(naive_utc, Local::now().offset().fix())
}

pub fn channel_id_from_tags(tags: &[nostr::Tag]) -> Option<nostr::EventId> {
    tags.iter().find_map(|tag| {
        if let nostr::Tag::Event(event_id, _, _) = tag {
            Some(event_id.to_owned())
        } else {
            None
        }
    })
}

pub fn contact_matches_search_full(contact: &DbContact, search: &str) -> bool {
    let ct_pubkey = contact
        .pubkey()
        .to_bech32()
        .unwrap_or(contact.pubkey().to_string());

    let pubkey_matches = ct_pubkey.to_lowercase().contains(&search.to_lowercase());

    let petname_matches = contact.get_petname().map_or(false, |petname| {
        petname.to_lowercase().contains(&search.to_lowercase())
    });

    let profile_name_matches = contact.get_profile_name().map_or(false, |name| {
        name.to_lowercase().contains(&search.to_lowercase())
    });

    let display_name_matches = contact.get_display_name().map_or(false, |display_name| {
        display_name.to_lowercase().contains(&search.to_lowercase())
    });

    pubkey_matches || petname_matches || profile_name_matches || display_name_matches
}

pub fn add_ellipsis_trunc(s: &str, max_length: usize) -> String {
    if s.chars().count() > max_length {
        let truncated = s.chars().take(max_length).collect::<String>();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

pub fn change_color_by_type(theme_type: ThemeType, color: iced::Color, amount: f32) -> iced::Color {
    match theme_type {
        ThemeType::Light => darken_color(color, amount),
        ThemeType::Dark => lighten_color(color, amount),
    }
}

pub fn lighten_color(mut color: iced::Color, amount: f32) -> iced::Color {
    color.r = (color.r + amount).min(1.0);
    color.g = (color.g + amount).min(1.0);
    color.b = (color.b + amount).min(1.0);
    color
}

pub fn darken_color(mut color: iced::Color, amount: f32) -> iced::Color {
    color.r = (color.r - amount).max(0.0);
    color.g = (color.g - amount).max(0.0);
    color.b = (color.b - amount).max(0.0);
    color
}

pub fn qr_code_handle(code: &str) -> Result<Handle, Error> {
    // Encode some data into bits.
    let code = match QrCode::new(code.as_bytes()) {
        Err(e) => {
            tracing::error!("Error creating QR code: {}", e);
            return Err(Error::QrError(e));
        }
        Ok(code) => code,
    };

    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();

    // Convert the Luma<u8> image into an Rgba<u8> image
    let rgba_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
            let pixel = image.get_pixel(x, y);
            let color = pixel[0];
            Rgba([color, color, color, 255]) // Use the grayscale color for each of RGB and set max alpha
        });

    // Get the dimensions
    let (width, height) = rgba_image.dimensions();
    // Get the raw bytes
    let bytes = rgba_image.into_raw();

    Ok(Handle::from_pixels(width, height, bytes)) // Pass the owned bytes
}

#[derive(Debug, Clone)]
pub struct NipData {
    pub number: u16,
    pub description: String,
    pub repo_link: String,
}
pub fn parse_nips_markdown(markdown_content: &str) -> Result<Vec<NipData>, Error> {
    let re = Regex::new(r"- \[NIP-(\d+): (.*?)\]\((\d+).md\)")?;
    let mut nip_data: Vec<_> = Vec::new();

    for line in markdown_content.lines() {
        if let Some(cap) = re.captures(line) {
            let nip_number = cap[1].parse::<u32>()?;
            let description = &cap[2];
            let repo_link = format!(
                "https://github.com/nostr-protocol/nips/blob/master/{:02}.md",
                nip_number
            );

            nip_data.push(NipData {
                description: description.to_string(),
                number: nip_number as u16,
                repo_link,
            });
        }
    }
    Ok(nip_data)
}
/// Hides the middle part of a string with "..."
pub fn hide_string(string: &str, open: usize) -> String {
    let chars: Vec<char> = string.chars().collect();
    let len = chars.len();

    // If open value is greater than half of the string length, return the entire string
    if open >= len / 2 {
        return string.to_string();
    }

    let prefix: String = chars.iter().take(open).collect();
    let suffix: String = chars.iter().rev().take(open).collect();

    format!("{}...{}", prefix, suffix.chars().rev().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nips_markdown() {
        let markdown_content = "
        - [NIP-01: Basic protocol flow description](01.md)
        - [NIP-02: Contact List and Petnames](02.md)
        ";

        let nips = parse_nips_markdown(markdown_content).unwrap();

        assert_eq!(nips.len(), 2);

        assert_eq!(nips[0].number, 1);
        assert_eq!(nips[0].description, "Basic protocol flow description");
        assert_eq!(
            nips[0].repo_link,
            "https://github.com/nostr-protocol/nips/blob/master/01.md"
        );

        assert_eq!(nips[1].number, 2);
        assert_eq!(nips[1].description, "Contact List and Petnames");
        assert_eq!(
            nips[1].repo_link,
            "https://github.com/nostr-protocol/nips/blob/master/02.md"
        );
    }

    #[test]
    fn test_hide_string() {
        // the string total chars is 13
        // 0 from each side turns into 0 chars, hide the entire string
        assert_eq!(hide_string("Hello, world!", 0), "...");

        // 2 from each side turns into 4 chars, hide 9 chars
        assert_eq!(hide_string("Hello, world!", 2), "He...d!");

        // 5 from each side turns into 10 chars, hide 3 chars
        assert_eq!(hide_string("Hello, world!", 5), "Hello...orld!");

        // 8 from each side turns into 16 chars, open the entire string
        assert_eq!(hide_string("Hello, world!", 8), "Hello, world!");
    }
}

// pub fn round_image(image: &mut ColorImage) {
//     // The radius to the edge of of the avatar circle
//     let edge_radius = image.size[0] as f32 / 2.0;
//     let edge_radius_squared = edge_radius * edge_radius;

//     for (pixnum, pixel) in image.pixels.iter_mut().enumerate() {
//         // y coordinate
//         let uy = pixnum / image.size[0];
//         let y = uy as f32;
//         let y_offset = edge_radius - y;

//         // x coordinate
//         let ux = pixnum % image.size[0];
//         let x = ux as f32;
//         let x_offset = edge_radius - x;

//         // The radius to this pixel (may be inside or outside the circle)
//         let pixel_radius_squared: f32 = x_offset * x_offset + y_offset * y_offset;

//         // If inside of the avatar circle
//         if pixel_radius_squared <= edge_radius_squared {
//             // squareroot to find how many pixels we are from the edge
//             let pixel_radius: f32 = pixel_radius_squared.sqrt();
//             let distance = edge_radius - pixel_radius;

//             // If we are within 1 pixel of the edge, we should fade, to
//             // antialias the edge of the circle. 1 pixel from the edge should
//             // be 100% of the original color, and right on the edge should be
//             // 0% of the original color.
//             if distance <= 1.0 {
//                 *pixel = Color32::from_rgba_premultiplied(
//                     (pixel.r() as f32 * distance) as u8,
//                     (pixel.g() as f32 * distance) as u8,
//                     (pixel.b() as f32 * distance) as u8,
//                     (pixel.a() as f32 * distance) as u8,
//                 );
//             }
//         } else {
//             // Outside of the avatar circle
//             *pixel = Color32::TRANSPARENT;
//         }
//     }
// }
