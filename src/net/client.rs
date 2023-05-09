use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use futures::TryStreamExt;
use futures_util::StreamExt;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use reqwest::Url;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::consts::APP_PROJECT_DIRS;
use crate::db::DbContact;
use crate::error::Error;

pub enum ImageKind {
    Profile,
    Banner,
}
impl ImageKind {
    pub fn to_str(&self, image_type: &str) -> String {
        match self {
            ImageKind::Profile => format!("profile_1.{}", image_type),
            ImageKind::Banner => format!("banner_1.{}", image_type),
        }
    }
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
        .ok_or(Error::RequestMissingContentType)?;

    if !content_type.to_str()?.starts_with("image/") {
        return Err(Error::ImageInvalidContentType(
            content_type.to_str()?.to_owned(),
        ));
    }

    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    let images_dir = dirs
        .config_dir()
        .join(IMAGES_FOLDER_NAME)
        .join(public_key.to_string());
    tokio::fs::create_dir_all(&images_dir).await?;

    // Extract the image type from the content type.
    let content_type_str = content_type.to_str()?;
    let image_type = content_type_str
        .split("/")
        .nth(1)
        .ok_or(Error::ImageInvalidContentType(content_type_str.to_owned()))?;

    let image_path = images_dir.join(kind.to_str(image_type));

    // Open the file for writing
    let mut dest = File::create(&image_path).await?;

    let mut stream = response.bytes_stream().map_err(Error::ReqwestStreamError);

    while let Some(item) = stream.next().await {
        let chunk = item?;
        dest.write_all(&chunk).await?;
    }

    Ok(image_path)
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
