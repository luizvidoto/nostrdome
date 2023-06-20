use std::path::PathBuf;

use crate::{
    net::{image_filename, ImageKind, ImageSize},
    utils::{event_hash_or_err, image_kind_or_err},
};
use nostr::EventId;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing JSON content into nostr::Metadata: {0}")]
    JsonToMetadata(String),

    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Event need to be confirmed")]
    NotConfirmedEvent(EventId),

    #[error("Not found path for kind: {0:?}")]
    NoPathForKind(ImageKind),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{1:?} cache not found for event_id: {0}")]
    ImageCacheNotFound(EventId, ImageKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDownloaded {
    pub path: PathBuf,
    pub kind: ImageKind,
    pub event_hash: EventId,
}
impl ImageDownloaded {
    pub fn sized_image(&self, size: ImageSize) -> PathBuf {
        let sized_file_name = image_filename(self.kind, size, "png");
        // replace filename with new
        self.path.with_file_name(sized_file_name)
    }
    pub async fn fetch(
        cache_pool: &SqlitePool,
        event_hash: &EventId,
        kind: ImageKind,
    ) -> Result<Option<ImageDownloaded>, Error> {
        Ok(sqlx::query_as::<_, ImageDownloaded>(
            "SELECT * FROM image_cache WHERE event_hash = ? AND kind = ?",
        )
        .bind(event_hash.to_string())
        .bind(kind.as_i32())
        .fetch_optional(cache_pool)
        .await?)
    }
    pub async fn insert(
        cache_pool: &SqlitePool,
        image: &ImageDownloaded,
    ) -> Result<ImageDownloaded, Error> {
        if let Some(cache) = Self::fetch(cache_pool, &image.event_hash, image.kind).await? {
            return Ok(cache);
        }

        let insert_query = r#"
                INSERT INTO image_cache (path, kind, event_hash) 
                VALUES (?, ?, ?)
            "#;

        sqlx::query(insert_query)
            .bind(&image.path.to_string_lossy())
            .bind(image.kind.as_i32())
            .bind(&image.event_hash.to_string())
            .execute(cache_pool)
            .await?;

        let cache = Self::fetch(cache_pool, &image.event_hash, image.kind)
            .await?
            .ok_or(Error::ImageCacheNotFound(
                image.event_hash.to_owned(),
                image.kind,
            ))?;

        Ok(cache)
    }

    pub async fn delete(
        cache_pool: &SqlitePool,
        event_hash: &EventId,
        kind: ImageKind,
    ) -> Result<(), Error> {
        match Self::fetch(cache_pool, event_hash, kind).await? {
            None => Err(Error::ImageCacheNotFound(event_hash.to_owned(), kind)),
            Some(cache) => {
                let delete_query = r#"DELETE FROM image_cache WHERE event_hash = ? AND kind = ?"#;

                delete_images(cache).await?;

                sqlx::query(delete_query)
                    .bind(&event_hash.to_string())
                    .bind(kind.as_i32())
                    .execute(cache_pool)
                    .await?;

                Ok(())
            }
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for ImageDownloaded {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let path: String = row.get("path");
        let path = PathBuf::from(path);

        let kind: i32 = row.get("kind");
        let kind = image_kind_or_err(kind, "kind")?;

        let event_hash: String = row.get("event_hash");
        let event_hash = event_hash_or_err(&event_hash, "event_hash")?;

        Ok(Self {
            path,
            kind,
            event_hash,
        })
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
