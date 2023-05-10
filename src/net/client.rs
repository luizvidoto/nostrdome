use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use futures::TryStreamExt;
use futures_util::StreamExt;
use image::ImageFormat;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use reqwest::Url;
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::consts::{APP_PROJECT_DIRS, SMALL_PROFILE_PIC_HEIGHT, SMALL_PROFILE_PIC_WIDTH};
use crate::error::Error;

pub enum ImageSize {
    Small,
    Large,
}
impl ImageSize {
    pub fn to_str(&self) -> &str {
        match self {
            ImageSize::Small => "small",
            ImageSize::Large => "large",
        }
    }
}

pub enum ImageKind {
    Profile,
    Banner,
}
impl ImageKind {
    pub fn to_str(&self, image_type: &str, image_size: ImageSize) -> String {
        match self {
            ImageKind::Profile => format!("profile_1_{}.{}", image_size.to_str(), image_type),
            ImageKind::Banner => format!("banner_1_{}.{}", image_size.to_str(), image_type),
        }
    }
}

pub fn get_png_image_path(base_path: &Path, kind: ImageKind, size: ImageSize) -> PathBuf {
    // Always use PNG as the image format for the resized images
    let image_type = "png";

    let file_name = kind.to_str(image_type, size);
    base_path.with_file_name(file_name)
}

pub async fn download_image(
    image_url: &str,
    public_key: &XOnlyPublicKey,
    kind: ImageKind,
) -> Result<PathBuf, Error> {
    let url = Url::parse(image_url)?;
    let response = reqwest::get(url.clone()).await?;

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
    let image_path = images_dir.join(kind.to_str(image_type, ImageSize::Large));
    let mut dest = File::create(&image_path).await?;
    let mut stream = response.bytes_stream().map_err(Error::ReqwestStreamError);
    while let Some(item) = stream.next().await {
        let chunk = item?;
        dest.write_all(&chunk).await?;
    }

    let small_image_path = images_dir.join(kind.to_str("png", ImageSize::Small));
    resize_and_save_image(
        &image_path,
        &small_image_path,
        SMALL_PROFILE_PIC_WIDTH,
        SMALL_PROFILE_PIC_HEIGHT,
    )?;

    Ok(image_path)
}

pub fn resize_and_save_image(
    input_path: &Path,
    output_path: &Path,
    desired_width: u32,
    desired_height: u32,
) -> Result<(), Error> {
    let image = image::open(input_path)?;

    let resized_image = image.resize(
        desired_width,
        desired_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Save the resized image as a PNG, regardless of the input format
    resized_image.save_with_format(output_path, ImageFormat::Png)?;

    Ok(())
}

// pub fn resize_and_save_image(
//     input_path: &Path,
//     output_path: &Path,
//     desired_width: u32,
//     desired_height: u32,
// ) -> Result<(), Error> {
//     let image = image::open(input_path)?;

//     let resized_image = image.resize(
//         desired_width,
//         desired_height,
//         image::imageops::FilterType::Lanczos3,
//     );

//     let extension = input_path.extension().ok_or(Error::ImageInvalidExtension(
//         input_path.to_string_lossy().into_owned(),
//     ))?;
//     let format = ImageFormat::from_extension(extension)
//         .ok_or_else(|| Error::ImageInvalidFormat(extension.to_string_lossy().into_owned()))?;

//     resized_image.save_with_format(output_path, format)?;

//     Ok(())
// }

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
