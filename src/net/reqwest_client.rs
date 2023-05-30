use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use futures::TryStreamExt;
use futures_util::StreamExt;
use image::ImageFormat;
use nostr::{secp256k1::XOnlyPublicKey, Url};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::consts::{
    APP_PROJECT_DIRS, MEDIUM_PROFILE_PIC_HEIGHT, MEDIUM_PROFILE_PIC_WIDTH,
    SMALL_PROFILE_PIC_HEIGHT, SMALL_PROFILE_PIC_WIDTH,
};

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
            ImageSize::Small => Some((SMALL_PROFILE_PIC_WIDTH, SMALL_PROFILE_PIC_HEIGHT)),
            ImageSize::Medium => Some((MEDIUM_PROFILE_PIC_WIDTH, MEDIUM_PROFILE_PIC_HEIGHT)),
            ImageSize::Original => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum ImageKind {
    Profile,
    Banner,
}
impl ImageKind {
    pub fn to_str(&self) -> &str {
        match self {
            ImageKind::Profile => "profile_1",
            ImageKind::Banner => "banner_1",
        }
    }
}

pub fn image_filename(kind: ImageKind, size: ImageSize, image_type: &str) -> String {
    format!("{}_{}.{}", kind.to_str(), size.to_str(), image_type)
}

pub fn sized_image(filename: &Path, kind: ImageKind, size: ImageSize) -> PathBuf {
    let new_file_name = image_filename(kind, size, "png");
    // replace filename with new
    filename.with_file_name(new_file_name)
}

pub struct ImageDownloaded {
    pub kind: ImageKind,
    pub public_key: XOnlyPublicKey,
    pub path: PathBuf,
}

pub async fn download_image(
    image_url: &str,
    public_key: &XOnlyPublicKey,
    kind: ImageKind,
) -> Result<ImageDownloaded, Error> {
    let image_url = Url::parse(image_url)?;
    let response = reqwest::get(image_url.clone()).await?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .ok_or(Error::RequestMissingContentType)
        .cloned()?;

    if !content_type.to_str()?.starts_with("image/") {
        return Err(Error::ImageInvalidContentType(
            content_type.to_str()?.to_owned(),
        ));
    }

    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    let images_dir = dirs
        .cache_dir()
        .join(IMAGES_FOLDER_NAME)
        .join(public_key.to_string());
    tokio::fs::create_dir_all(&images_dir).await?;

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
        public_key: public_key.clone(),
        path: original_path,
    })
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
