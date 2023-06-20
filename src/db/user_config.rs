use crate::{
    net::ntp::{correct_time_with_offset, system_now_microseconds, system_time_to_naive_utc},
    utils::url_or_err,
};

use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("System time before unix epoch")]
    SystemTimeBeforeUnixEpoch,

    #[error("Error converting to NaiveDateTime UTC: {0}")]
    ConvertingToNaiveUtc(String),
}

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub recommended_relay: Option<Url>,
    pub has_logged_in: bool,
    pub ntp_offset: i64,
}

impl UserConfig {
    pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        tracing::debug!("setup_user_config");
        let query = r#"
            INSERT INTO user_config 
                (id, has_logged_in, ntp_offset, recommended_relay) 
            VALUES (1, 0, 0, "");
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

    pub(crate) async fn update_ntp_offset(pool: &SqlitePool, ntp_time: u64) -> Result<i64, Error> {
        tracing::debug!("update_ntp_offset");

        let system_total_microseconds =
            system_now_microseconds().map_err(|_| Error::SystemTimeBeforeUnixEpoch)?;

        let offset = ntp_time as i64 - system_total_microseconds as i64;

        let query = "UPDATE user_config SET ntp_offset = ?1 WHERE id = 1;";

        sqlx::query(query).bind(offset).execute(pool).await?;

        Ok(offset)
    }

    pub(crate) async fn get_corrected_time(pool: &SqlitePool) -> Result<NaiveDateTime, Error> {
        tracing::debug!("get_corrected_time");

        // Query the database for the offset
        let query = "SELECT ntp_offset FROM user_config WHERE id = 1;";
        let offset: i64 = sqlx::query_scalar(query).fetch_one(pool).await?;

        // Get the current system time in total microseconds
        let system_total_microseconds =
            system_now_microseconds().map_err(|_| Error::SystemTimeBeforeUnixEpoch)?;

        // Correct the system time with the offset and convert to NaiveDateTime
        let corrected_system_time = correct_time_with_offset(system_total_microseconds, offset);
        let corrected_time = system_time_to_naive_utc(corrected_system_time)
            .map_err(|e| Error::ConvertingToNaiveUtc(e.to_string()))?;

        Ok(corrected_time)
    }
    pub(crate) async fn get_ntp_offset(pool: &SqlitePool) -> Result<i64, Error> {
        let query = "SELECT ntp_offset FROM user_config WHERE id = 1;";
        let offset: i64 = sqlx::query_scalar(query).fetch_one(pool).await?;
        Ok(offset)
    }

    pub(crate) async fn set_relay(pool: &SqlitePool, recommended_relay: &Url) -> Result<(), Error> {
        let query = "UPDATE user_config SET recommended_relay = ? WHERE id = 1;";
        sqlx::query(query)
            .bind(recommended_relay.to_string())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_relay(pool: &SqlitePool) -> Result<Option<Url>, Error> {
        let query = "SELECT recommended_relay FROM user_config WHERE id = 1;";
        let recommended_relay: String = sqlx::query_scalar(query).fetch_one(pool).await?;
        let recommended_relay = Url::parse(&recommended_relay).ok();
        Ok(recommended_relay)
    }
}

impl sqlx::FromRow<'_, SqliteRow> for UserConfig {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let recommended_relay: String = row.try_get("recommended_relay")?;
        let recommended_relay = url_or_err(&recommended_relay, "recommended_relay").ok();

        Ok(Self {
            has_logged_in: row.try_get::<bool, &str>("has_logged_in")?,
            ntp_offset: row.try_get::<i64, &str>("ntp_offset")?,
            recommended_relay,
        })
    }
}
