use std::path::PathBuf;

use crate::{
    error::Error,
    utils::{millis_to_naive_or_err, profile_meta_or_err},
};

use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub has_logged_in: bool,
    pub profile_meta: Option<nostr_sdk::Metadata>,
    pub profile_meta_last_update: Option<NaiveDateTime>,
    pub local_profile_image: Option<PathBuf>,
    pub local_banner_image: Option<PathBuf>,
    pub main_subscription_id: Option<nostr_sdk::SubscriptionId>,
}

impl UserConfig {
    pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        tracing::info!("setup_user_config");
        let query = r#"
            INSERT INTO user_config 
                (id, has_logged_in, profile_meta, 
                profile_meta_last_update, local_profile_image, 
                local_banner_image, main_subscription_id) 
            VALUES (1, 0, "", 0, "", "", "");
        "#;
        sqlx::query(query).execute(pool).await?;
        Ok(())
    }

    pub async fn store_first_login(pool: &SqlitePool) -> Result<(), Error> {
        tracing::info!("store_first_login");
        let query = "UPDATE user_config SET has_logged_in = 1 WHERE id = 1;";
        sqlx::query(query).execute(pool).await?;
        Ok(())
    }

    pub async fn query_has_logged_in(pool: &SqlitePool) -> Result<bool, Error> {
        tracing::debug!("query_has_logged_in");
        let query = "SELECT has_logged_in FROM user_config;";
        let has_logged_in: Option<i32> = sqlx::query(query)
            .map(|row: SqliteRow| row.get(0))
            .fetch_optional(pool)
            .await?;
        let has_logged_in = has_logged_in.unwrap_or(0);
        Ok(has_logged_in != 0)
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Self, Error> {
        tracing::info!("Fetch UserConfig");
        let query = "SELECT * FROM user_config WHERE id = 1;";
        let user = sqlx::query_as::<_, UserConfig>(query)
            .fetch_one(pool)
            .await?;
        Ok(user)
    }

    pub async fn update_user_metadata_if_newer(
        pool: &SqlitePool,
        profile_meta: &nostr_sdk::Metadata,
        last_update: NaiveDateTime,
    ) -> Result<(), Error> {
        if Self::should_update_user_metadata(pool, &last_update).await? {
            Self::update_user_metadata(profile_meta, &last_update, pool).await?;
        }
        Ok(())
    }

    pub async fn update_user_metadata(
        profile_meta: &nostr_sdk::Metadata,
        last_update: &NaiveDateTime,
        pool: &SqlitePool,
    ) -> Result<(), Error> {
        let query =
            "UPDATE user_config SET profile_meta=?, profile_meta_last_update=? WHERE id = 1;";
        sqlx::query(query)
            .bind(&profile_meta.as_json())
            .bind(last_update.timestamp_millis())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn should_update_user_metadata(
        pool: &SqlitePool,
        last_update: &NaiveDateTime,
    ) -> Result<bool, Error> {
        let user = Self::fetch(pool).await?;
        let should_update = match user.profile_meta_last_update {
            Some(previous_update) if &previous_update > last_update => {
                tracing::warn!("Cant update user profile with older data");
                false
            }
            _ => true,
        };

        Ok(should_update)
    }

    pub(crate) async fn update_user_profile_picture(
        pool: &SqlitePool,
        path: &PathBuf,
    ) -> Result<(), Error> {
        let query = "UPDATE user_config SET local_profile_image=? WHERE id = 1;";
        sqlx::query(query).bind(path.to_str()).execute(pool).await?;
        Ok(())
    }

    pub(crate) async fn update_user_banner_picture(
        pool: &SqlitePool,
        path: &PathBuf,
    ) -> Result<(), Error> {
        let query = "UPDATE user_config SET local_banner_image=? WHERE id = 1;";
        sqlx::query(query).bind(path.to_str()).execute(pool).await?;
        Ok(())
    }

    // insert main_subscription_id
    pub(crate) async fn update_main_subcription_id(
        pool: &SqlitePool,
        subscription_id: nostr_sdk::SubscriptionId,
    ) -> Result<(), Error> {
        let query = "UPDATE user_config SET main_subscription_id=? WHERE id = 1;";
        sqlx::query(query)
            .bind(subscription_id.to_string())
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for UserConfig {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let profile_meta: String = row.try_get("profile_meta")?;
        let profile_meta = profile_meta_or_err(&profile_meta, "profile_meta").ok();

        let profile_meta_last_update: i64 = row.try_get("profile_meta_last_update")?;
        let profile_meta_last_update =
            millis_to_naive_or_err(profile_meta_last_update, "profile_meta_last_update").ok();

        let local_profile_image: Option<String> = row.get("local_profile_image");
        let local_profile_image = local_profile_image.map(|path| PathBuf::from(path));

        let local_banner_image: Option<String> = row.get("local_banner_image");
        let local_banner_image = local_banner_image.map(|path| PathBuf::from(path));

        let main_subscription_id: Option<String> = row.get("main_subscription_id");
        let main_subscription_id = main_subscription_id.map(|s| nostr_sdk::SubscriptionId::new(s));

        Ok(Self {
            profile_meta,
            profile_meta_last_update,
            local_profile_image,
            local_banner_image,
            has_logged_in: row.try_get::<bool, &str>("has_logged_in")?,
            main_subscription_id,
        })
    }
}
