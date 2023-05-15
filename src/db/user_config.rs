use std::path::PathBuf;

use crate::{
    error::Error,
    ntp::{correct_time_with_offset, system_now_total_microseconds, system_time_to_naive_utc},
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
    pub ntp_offset: i64,
}

impl UserConfig {
    pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        tracing::debug!("setup_user_config");
        let query = r#"
            INSERT INTO user_config 
                (id, has_logged_in, profile_meta, 
                profile_meta_last_update, local_profile_image, 
                local_banner_image, ntp_offset) 
            VALUES (1, 0, "", 0, "", "", 0);
        "#;
        sqlx::query(query).execute(pool).await?;
        Ok(())
    }

    pub async fn store_first_login(pool: &SqlitePool) -> Result<(), Error> {
        tracing::debug!("store_first_login");
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
        tracing::debug!("Fetch UserConfig");
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
        tracing::debug!("update_user_metadata_if_newer");
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
        tracing::debug!("update_user_metadata");
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
        tracing::debug!("should_update_user_metadata");
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
        tracing::debug!("update_user_profile_picture");
        let query = "UPDATE user_config SET local_profile_image=? WHERE id = 1;";
        sqlx::query(query).bind(path.to_str()).execute(pool).await?;
        Ok(())
    }

    pub(crate) async fn update_user_banner_picture(
        pool: &SqlitePool,
        path: &PathBuf,
    ) -> Result<(), Error> {
        tracing::debug!("update_user_banner_picture");
        let query = "UPDATE user_config SET local_banner_image=? WHERE id = 1;";
        sqlx::query(query).bind(path.to_str()).execute(pool).await?;
        Ok(())
    }

    pub(crate) async fn update_ntp_offset(
        pool: &SqlitePool,
        ntp_total_microseconds: u64,
    ) -> Result<(), Error> {
        tracing::debug!("update_ntp_offset");

        let system_total_microseconds =
            system_now_total_microseconds().map_err(|_| Error::SystemTimeBeforeUnixEpoch)?;

        let offset = ntp_total_microseconds as i64 - system_total_microseconds as i64;

        let query = "UPDATE user_config SET ntp_offset = ?1 WHERE id = 1;";
        sqlx::query(query).bind(offset).execute(pool).await?;
        Ok(())
    }

    pub(crate) async fn get_corrected_time(pool: &SqlitePool) -> Result<NaiveDateTime, Error> {
        tracing::debug!("get_corrected_time");

        // Query the database for the offset
        let query = "SELECT ntp_offset FROM user_config WHERE id = 1;";
        let offset: i64 = sqlx::query_scalar(query).fetch_one(pool).await?;

        // Get the current system time in total microseconds
        let system_total_microseconds =
            system_now_total_microseconds().map_err(|_| Error::SystemTimeBeforeUnixEpoch)?;

        // Correct the system time with the offset and convert to NaiveDateTime
        let corrected_system_time = correct_time_with_offset(system_total_microseconds, offset);
        let corrected_time = system_time_to_naive_utc(corrected_system_time)?;

        Ok(corrected_time)
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

        Ok(Self {
            profile_meta,
            profile_meta_last_update,
            local_profile_image,
            local_banner_image,
            has_logged_in: row.try_get::<bool, &str>("has_logged_in")?,
            ntp_offset: row.try_get::<i64, &str>("ntp_offset")?,
        })
    }
}
