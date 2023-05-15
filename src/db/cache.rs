use std::path::PathBuf;

use chrono::NaiveDateTime;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::Error,
    net::ImageKind,
    utils::{millis_to_naive_or_err, public_key_or_err, url_or_err},
};

pub struct Cache {
    pub public_key: XOnlyPublicKey,
    pub updated_at: NaiveDateTime,
    pub profile_image_url: Option<nostr_sdk::Url>,
    pub profile_image_path: Option<PathBuf>,
    pub banner_image_url: Option<nostr_sdk::Url>,
    pub banner_image_path: Option<PathBuf>,
}
impl Cache {
    pub async fn fetch_by_public_key(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
    ) -> Result<Option<Cache>, Error> {
        let query = "SELECT * FROM cache_history WHERE public_key = ?;";
        let result = sqlx::query_as::<_, Cache>(query)
            .bind(&public_key.to_string())
            .fetch_optional(cache_pool)
            .await?;
        Ok(result)
    }

    pub async fn insert_by_public_key(
        cache_pool: &SqlitePool,
        public_key: &XOnlyPublicKey,
        event_date: &NaiveDateTime,
        kind: ImageKind,
        path: &PathBuf,
        url: &nostr_sdk::Url,
    ) -> Result<(), Error> {
        let (img_url_column, img_path_column) = match kind {
            ImageKind::Profile => ("profile_image_url", "profile_image_path"),
            ImageKind::Banner => ("banner_image_url", "banner_image_path"),
        };
        let mut tx = cache_pool.begin().await?;

        let update_query = format!(
            r#"UPDATE cache_history 
            SET updated_at = ?, {} = ?, {} = ?
            WHERE public_key = ?
        "#,
            img_url_column, img_path_column
        );
        let rows_affected = sqlx::query(&update_query)
            .bind(&event_date.timestamp_millis())
            .bind(&url.to_string())
            .bind(
                &path
                    .to_str()
                    .ok_or_else(|| Error::InvalidPath(path.clone()))?,
            )
            .bind(&public_key.to_string())
            .execute(&mut tx)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            let insert_query = format!(
                r#"INSERT INTO cache_history
                    (public_key, updated_at, {}, {}) 
                    VALUES (?, ?, ?, ?)
                "#,
                img_url_column, img_path_column
            );
            sqlx::query(&insert_query)
                .bind(&public_key.to_string())
                .bind(&event_date.timestamp_millis())
                .bind(&url.to_string())
                .bind(
                    &path
                        .to_str()
                        .ok_or_else(|| Error::InvalidPath(path.clone()))?,
                )
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) fn get_url(&self, kind: ImageKind) -> Option<nostr_sdk::Url> {
        match kind {
            ImageKind::Profile => self.profile_image_url.clone(),
            ImageKind::Banner => self.banner_image_url.clone(),
        }
    }

    pub(crate) fn get_path(&self, kind: ImageKind) -> Option<PathBuf> {
        match kind {
            ImageKind::Profile => self.profile_image_path.clone(),
            ImageKind::Banner => self.banner_image_path.clone(),
        }
    }
}

impl sqlx::FromRow<'_, SqliteRow> for Cache {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let public_key = row.try_get::<String, &str>("public_key")?;
        let public_key = public_key_or_err(&public_key, "public_key")?;

        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;

        let profile_image_path: Option<String> = row.get("profile_image_path");
        let profile_image_path = profile_image_path.map(|path| PathBuf::from(path));

        let banner_image_path: Option<String> = row.get("banner_image_path");
        let banner_image_path = banner_image_path.map(|path| PathBuf::from(path));

        let profile_image_url = row
            .get::<Option<String>, &str>("profile_image_url")
            .map(|url| url_or_err(&url, "profile_image_url"))
            .transpose()?;

        let banner_image_url = row
            .get::<Option<String>, &str>("banner_image_url")
            .map(|url| url_or_err(&url, "banner_image_url"))
            .transpose()?;

        Ok(Self {
            public_key,
            updated_at,
            profile_image_path,
            profile_image_url,
            banner_image_path,
            banner_image_url,
        })
    }
}
