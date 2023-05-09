use crate::{
    error::Error,
    utils::{millis_to_naive_or_err, profile_meta_or_err},
};

use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

#[derive(Debug, Clone)]
struct UserConfig {
    has_logged_in: bool,
    profile_meta: Option<nostr_sdk::Metadata>,
    profile_meta_last_update: Option<NaiveDateTime>,
}

pub async fn store_first_login(pool: &SqlitePool) -> Result<(), Error> {
    tracing::info!("store_first_login");
    let query = "UPDATE user_config SET has_logged_in = 1 WHERE id = 1;";
    sqlx::query(query).execute(pool).await?;
    Ok(())
}

pub async fn query_has_logged_in(pool: &SqlitePool) -> Result<bool, Error> {
    let query = "SELECT has_logged_in FROM user_config;";
    let has_logged_in: Option<i32> = sqlx::query(query)
        .map(|row: SqliteRow| row.get(0))
        .fetch_optional(pool)
        .await?;
    let has_logged_in = has_logged_in.unwrap_or(0);
    Ok(has_logged_in != 0)
}

pub async fn fetch_user_meta(pool: &SqlitePool) -> Result<Option<nostr_sdk::Metadata>, Error> {
    let user = fetch_user_config(pool).await?;
    Ok(user.profile_meta)
}

pub async fn update_user_meta(
    pool: &SqlitePool,
    profile_meta: &nostr_sdk::Metadata,
    last_update: NaiveDateTime,
) -> Result<(), Error> {
    let user = fetch_user_config(pool).await?;
    if let Some(previous_update) = user.profile_meta_last_update {
        if previous_update > last_update {
            tracing::info!("Cant update user profile with older data");
            return Ok(());
        }
    }

    let query = "UPDATE user_config SET profile_meta=?, profile_meta_last_update=? WHERE id = 1;";
    sqlx::query(query)
        .bind(&profile_meta.as_json())
        .bind(last_update.timestamp_millis())
        .execute(pool)
        .await?;
    Ok(())
}

async fn fetch_user_config(pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<UserConfig, Error> {
    let query = "SELECT * FROM user_config WHERE id = 1;";
    let user = sqlx::query_as::<_, UserConfig>(query)
        .fetch_one(pool)
        .await?;
    Ok(user)
}
pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    tracing::info!("setup_user_config");
    let query = r#"INSERT INTO user_config (id, has_logged_in, profile_meta, profile_meta_last_update) VALUES (1, 0, "", 0);"#;
    sqlx::query(query).execute(pool).await?;
    Ok(())
}

impl sqlx::FromRow<'_, SqliteRow> for UserConfig {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let profile_meta: String = row.try_get("profile_meta")?;
        let profile_meta = profile_meta_or_err(&profile_meta, "profile_meta").ok();

        let profile_meta_last_update: i64 = row.try_get("profile_meta_last_update")?;
        let profile_meta_last_update =
            millis_to_naive_or_err(profile_meta_last_update, "profile_meta_last_update").ok();

        Ok(UserConfig {
            profile_meta,
            profile_meta_last_update,
            has_logged_in: row.try_get::<bool, &str>("has_logged_in")?,
        })
    }
}
