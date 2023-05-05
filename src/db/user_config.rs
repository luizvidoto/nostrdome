use crate::{error::Error, utils::profile_meta_or_err};

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

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

pub async fn fetch_user_meta(
    pool: &SqlitePool,
) -> Result<Option<nostr_sdk::Metadata>, sqlx::Error> {
    let query = "SELECT profile_meta FROM user_config;";
    let profile_meta: Option<String> = sqlx::query(query)
        .map(|row: SqliteRow| row.get(0))
        .fetch_optional(pool)
        .await?;

    let profile_meta = profile_meta
        .as_ref()
        .map(|json| profile_meta_or_err(json, "profile_meta"))
        .transpose()?;
    Ok(profile_meta)
}

pub async fn update_user_meta(
    pool: &SqlitePool,
    profile_meta: &nostr_sdk::Metadata,
) -> Result<(), sqlx::Error> {
    let query = "UPDATE user_config SET profile_meta=? WHERE id = 1;";
    sqlx::query(query)
        .bind(&profile_meta.as_json())
        .execute(pool)
        .await?;
    Ok(())
}
pub async fn setup_user_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    tracing::info!("setup_user_config");
    let query = "INSERT INTO user_config (id, has_logged_in) VALUES (1, 0);";
    sqlx::query(query).execute(pool).await?;
    Ok(())
}
