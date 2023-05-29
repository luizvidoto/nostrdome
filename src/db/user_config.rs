use crate::net::ntp::{
    correct_time_with_offset, system_now_total_microseconds, system_time_to_naive_utc,
};

use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("System time before unix epoch")]
    SystemTimeBeforeUnixEpoch,

    #[error("{0}")]
    FromNtpError(#[from] crate::net::ntp::Error),
}

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub has_logged_in: bool,
    pub ntp_offset: i64,
}

impl UserConfig {
    pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        tracing::debug!("setup_user_config");
        let query = r#"
            INSERT INTO user_config 
                (id, has_logged_in, ntp_offset) 
            VALUES (1, 0, 0);
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

    pub(crate) async fn update_ntp_offset(
        pool: &SqlitePool,
        total_microseconds: u64,
    ) -> Result<(), Error> {
        tracing::debug!("update_ntp_offset");

        let system_total_microseconds =
            system_now_total_microseconds().map_err(|_| Error::SystemTimeBeforeUnixEpoch)?;

        let offset = total_microseconds as i64 - system_total_microseconds as i64;

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
        Ok(Self {
            has_logged_in: row.try_get::<bool, &str>("has_logged_in")?,
            ntp_offset: row.try_get::<i64, &str>("ntp_offset")?,
        })
    }
}
