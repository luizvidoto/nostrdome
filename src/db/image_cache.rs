use std::path::PathBuf;

use crate::{
    net::{image_filename, ImageKind, ImageSize},
    utils::{image_kind_or_err, url_or_err},
};
use nostr::EventId;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing JSON content into nostr::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("Not found path for kind: {0:?}")]
    NoPathForKind(ImageKind),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image cache not found for url: {0}")]
    ImageCacheNotFound(Url),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDownloaded {
    pub url: Url,
    pub path: PathBuf,
    pub kind: ImageKind,
}
impl ImageDownloaded {
    pub async fn fetch(
        cache_pool: &SqlitePool,
        url: &Url,
    ) -> Result<Option<ImageDownloaded>, Error> {
        Ok(
            sqlx::query_as::<_, ImageDownloaded>("SELECT * FROM image_cache WHERE url = ?")
                .bind(url.to_string())
                .fetch_optional(cache_pool)
                .await?,
        )
    }
    pub async fn insert(
        cache_pool: &SqlitePool,
        image: &ImageDownloaded,
    ) -> Result<ImageDownloaded, Error> {
        if let Some(cache) = Self::fetch(cache_pool, &image.url).await? {
            return Ok(cache);
        }

        let insert_query = r#"
                INSERT INTO image_cache (url, path, kind) 
                VALUES (?, ?, ?)
            "#;

        sqlx::query(&insert_query)
            .bind(&image.url.to_string())
            .bind(&image.path.to_string_lossy())
            .bind(&image.kind.as_i32())
            .execute(cache_pool)
            .await?;

        let cache = Self::fetch(cache_pool, &image.url)
            .await?
            .ok_or(Error::ImageCacheNotFound(image.url.to_owned()))?;

        Ok(cache)
    }

    pub async fn delete(cache_pool: &SqlitePool, url: &Url) -> Result<(), Error> {
        match Self::fetch(cache_pool, url).await? {
            None => return Err(Error::ImageCacheNotFound(url.to_owned())),
            Some(cache) => {
                let delete_query = r#"DELETE FROM image_cache WHERE url = ?"#;

                delete_images(cache).await?;

                sqlx::query(&delete_query)
                    .bind(&url.to_string())
                    .execute(cache_pool)
                    .await?;

                Ok(())
            }
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for ImageDownloaded {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let url = row.try_get::<String, &str>("url")?;
        let url = url_or_err(&url, "url")?;

        let path: String = row.get("path");
        let path = PathBuf::from(path);

        let kind: i32 = row.get("kind");
        let kind = image_kind_or_err(kind, "kind")?;

        Ok(Self { url, path, kind })
    }
}

async fn delete_images(cache: ImageDownloaded) -> Result<(), Error> {
    tokio::fs::remove_file(cache.path).await?;

    match cache.kind {
        ImageKind::Profile => {
            let med_path = image_filename(cache.kind, ImageSize::Medium, "png");
            tokio::fs::remove_file(med_path).await?;

            let sm_path = image_filename(cache.kind, ImageSize::Small, "png");
            tokio::fs::remove_file(sm_path).await?;
        }
        ImageKind::Banner => {}
        ImageKind::Channel => {}
    }

    Ok(())
}
