use base64::engine::general_purpose;
use base64::{alphabet, engine, Engine};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use futures::TryStreamExt;
use futures_util::StreamExt;
use image::io::Reader;
use image::{DynamicImage, ImageFormat};
use nostr::{EventId, Url};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Cursor;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::consts::{
    APP_PROJECT_DIRS, MEDIUM_PROFILE_IMG_HEIGHT, MEDIUM_PROFILE_IMG_WIDTH,
    SMALL_PROFILE_IMG_HEIGHT, SMALL_PROFILE_IMG_WIDTH,
};
use crate::db::ImageDownloaded;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid URL: \"{0}\"")]
    InvalidUrl(#[from] url::ParseError),

    #[error("{0}")]
    FromToStrError(#[from] reqwest::header::ToStrError),

    #[error("Image error: {0}")]
    FromImageError(#[from] image::error::ImageError),

    #[error("Invalid image size: {0:?}")]
    InvalidImageSize(ImageSize),

    #[error("Request error: {0}")]
    FromReqwestError(#[from] reqwest::Error),

    #[error("Not found any releases")]
    RequestReleaseNotFound,

    #[error("GITHUB_TOKEN not found")]
    GitHubTokenNotFound,

    #[error("Missing content-type header")]
    RequestMissingContentType,

    #[error("Invalid content-type header: {0}")]
    ImageInvalidContentType(String),

    #[error("Not found project directory")]
    NotFoundProjectDirectory,

    #[error("Request error: {0}")]
    ReqwestStreamError(reqwest::Error),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid image kind")]
    InvalidImageKind,

    #[error("Invalid base64 encoded image")]
    InvalidBase64,

    #[error("Base64 decode error: {0}")]
    FromDecodeError(#[from] base64::DecodeError),

    #[error("Invalid image type: {0}")]
    InvalidImageType(String),
}

#[derive(Debug, Clone, Copy)]
pub enum ImageSize {
    Small,
    Medium,
    Original,
}
impl ImageSize {
    pub fn to_str(&self) -> &str {
        match self {
            ImageSize::Small => "small",
            ImageSize::Medium => "medium",
            ImageSize::Original => "",
        }
    }
    pub fn get_width_height(&self) -> Option<(u32, u32)> {
        match self {
            ImageSize::Small => Some((
                SMALL_PROFILE_IMG_WIDTH as u32,
                SMALL_PROFILE_IMG_HEIGHT as u32,
            )),
            ImageSize::Medium => Some((
                MEDIUM_PROFILE_IMG_WIDTH as u32,
                MEDIUM_PROFILE_IMG_HEIGHT as u32,
            )),
            ImageSize::Original => None,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageKind {
    Profile,
    Banner,
    Channel,
}
impl ImageKind {
    pub fn to_str(&self) -> &str {
        match self {
            ImageKind::Profile => "profile_1",
            ImageKind::Banner => "banner_1",
            ImageKind::Channel => "channel_1",
        }
    }
    pub fn as_i32(&self) -> i32 {
        match self {
            ImageKind::Profile => 1,
            ImageKind::Banner => 2,
            ImageKind::Channel => 3,
        }
    }
    pub fn from_i32(i: i32) -> Result<ImageKind, Error> {
        match i {
            1 => Ok(ImageKind::Profile),
            2 => Ok(ImageKind::Banner),
            3 => Ok(ImageKind::Channel),
            _ => Err(Error::InvalidImageKind),
        }
    }
}

pub fn image_filename(kind: ImageKind, size: ImageSize, image_type: &str) -> String {
    format!("{}_{}.{}", kind.to_str(), size.to_str(), image_type)
}

pub async fn download_image(
    image_url: &str,
    event_hash: &EventId,
    identifier: &str,
    kind: ImageKind,
) -> Result<ImageDownloaded, Error> {
    if image_url.starts_with("data:image/") {
        parse_base64(image_url, event_hash, identifier, kind).await
    } else {
        download_image_url(image_url, event_hash, identifier, kind).await
    }
}

async fn get_dirs(identifier: &str) -> Result<PathBuf, Error> {
    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    let images_dir = dirs.cache_dir().join(IMAGES_FOLDER_NAME).join(identifier);
    tokio::fs::create_dir_all(&images_dir).await?;
    Ok(images_dir)
}

async fn parse_base64(
    image_url: &str,
    event_hash: &EventId,
    identifier: &str,
    kind: ImageKind,
) -> Result<ImageDownloaded, Error> {
    let image_type = image_type_from_base64(image_url).ok_or(Error::InvalidBase64)?;
    let decoded = engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD)
        .decode(image_url)?;

    let images_dir = get_dirs(identifier).await?;
    let original_path = images_dir.join(image_filename(kind, ImageSize::Original, image_type));
    let image_format = ImageFormat::from_extension(image_type)
        .ok_or(Error::InvalidImageType(image_type.to_owned()))?;

    let cursor = Cursor::new(decoded);
    let mut reader = Reader::new(cursor);
    reader.set_format(image_format);

    let image = reader.decode()?;

    image.save_with_format(&original_path, image_format)?;
    save_dynamic_image(&images_dir, &image, kind, ImageSize::Medium)?;
    save_dynamic_image(&images_dir, &image, kind, ImageSize::Small)?;

    Ok(ImageDownloaded {
        kind,
        path: original_path,
        event_hash: event_hash.to_owned(),
    })
}

async fn download_image_url(
    image_url: &str,
    event_hash: &EventId,
    identifier: &str,
    kind: ImageKind,
) -> Result<ImageDownloaded, Error> {
    let image_url = Url::parse(image_url)?;
    let response = reqwest::get(image_url.clone()).await?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .ok_or(Error::RequestMissingContentType)
        .cloned()?;

    //TODO: builder error for url when BASE64

    if !content_type.to_str()?.starts_with("image/") {
        return Err(Error::ImageInvalidContentType(
            content_type.to_str()?.to_owned(),
        ));
    }

    let images_dir = get_dirs(identifier).await?;

    // Extract the image type from the content type.
    let content_type_str = content_type.to_str()?;
    let image_type = content_type_str
        .split("/")
        .nth(1)
        .ok_or(Error::ImageInvalidContentType(content_type_str.to_owned()))?;
    let original_path = images_dir.join(image_filename(kind, ImageSize::Original, image_type));
    let mut dest = File::create(&original_path).await?;
    let mut stream = response.bytes_stream().map_err(Error::ReqwestStreamError);
    while let Some(item) = stream.next().await {
        let chunk = item?;
        dest.write_all(&chunk).await?;
    }

    resize_and_save_image(&original_path, &images_dir, kind, ImageSize::Medium)?;
    resize_and_save_image(&original_path, &images_dir, kind, ImageSize::Small)?;

    Ok(ImageDownloaded {
        kind,
        path: original_path,
        event_hash: event_hash.to_owned(),
    })
}

pub fn save_dynamic_image(
    images_dir: &PathBuf,
    image: &DynamicImage,
    kind: ImageKind,
    size: ImageSize,
) -> Result<(), Error> {
    let output_filename = image_filename(kind, size, "png");
    tracing::debug!("file: {}", &output_filename);
    tracing::debug!("resizing: {} - size: {}", kind.to_str(), size.to_str());
    let output_path = images_dir.join(output_filename);
    let (width, height) = size
        .get_width_height()
        .ok_or(Error::InvalidImageSize(size.clone()))?;
    let resized_image = image.resize(width, height, image::imageops::FilterType::Lanczos3);

    // Save the resized image as a PNG, regardless of the input format
    resized_image.save_with_format(output_path, ImageFormat::Png)?;

    Ok(())
}

pub fn resize_and_save_image(
    original_path: &PathBuf,
    images_dir: &PathBuf,
    kind: ImageKind,
    size: ImageSize,
) -> Result<(), Error> {
    let output_filename = image_filename(kind, size, "png");
    tracing::debug!("file: {}", &output_filename);
    tracing::debug!("resizing: {} - size: {}", kind.to_str(), size.to_str());
    let output_path = images_dir.join(output_filename);
    let image = image::open(original_path)?;
    let (width, height) = size
        .get_width_height()
        .ok_or(Error::InvalidImageSize(size.clone()))?;
    let resized_image = image.resize(width, height, image::imageops::FilterType::Lanczos3);

    // Save the resized image as a PNG, regardless of the input format
    resized_image.save_with_format(output_path, ImageFormat::Png)?;

    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
}

pub async fn fetch_latest_version(client: reqwest::Client) -> Result<String, Error> {
    let token = env::var("GITHUB_TOKEN").map_err(|_| Error::GitHubTokenNotFound)?;
    let url = "https://api.github.com/repos/luizvidoto/nostrtalk/releases";
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", token))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    response.error_for_status_ref()?;

    let releases: Vec<GitHubRelease> = response.json().await?;

    // Ordena as releases por data de criação
    let mut sorted_releases: Vec<&GitHubRelease> = releases.iter().collect();
    sorted_releases.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let first_release = sorted_releases
        .get(0)
        .ok_or(Error::RequestReleaseNotFound)?;
    Ok(first_release.tag_name.clone())
}

const IMAGES_FOLDER_NAME: &str = "images";

fn image_type_from_base64(s: &str) -> Option<&str> {
    let parts: Vec<&str> = s.split(';').collect();
    if parts.len() > 1 {
        let mime_parts: Vec<&str> = parts[0].split(':').collect();
        if mime_parts.len() > 1 {
            let type_parts: Vec<&str> = mime_parts[1].split('/').collect();
            if type_parts.len() > 1 {
                return Some(type_parts[1]);
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_base64_image_url() {
        let base64_image_url = "data:image/jpeg;base64,/9j/4AAQSkZ...";
        assert_eq!(image_type_from_base64(base64_image_url), Some("jpeg"));

        let base64_image_url = "data:image/png;base64,iVBORw0...";
        assert_eq!(image_type_from_base64(base64_image_url), Some("png"));
    }

    #[test]
    fn test_invalid_base64_image_url() {
        let base64_image_url = "image/jpeg;base64,/9j/4AAQSkZ...";
        assert_eq!(image_type_from_base64(base64_image_url), None);

        let base64_image_url = "data:image/jpeg,/9j/4AAQSkZ...";
        assert_eq!(image_type_from_base64(base64_image_url), None);

        let base64_image_url = "/9j/4AAQSkZ...";
        assert_eq!(image_type_from_base64(base64_image_url), None);
    }
}
