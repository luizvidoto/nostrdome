use ns_client::RelayInformation;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use thiserror::Error;
use url::Url;

use crate::utils::url_or_err;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid URL: \"{0}\"")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Relay not found: {0}")]
    RelayNotFound(String),

    #[error("Serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DbRelay {
    pub id: i32,
    pub url: Url,
    pub read: bool,
    pub write: bool,
    pub advertise: bool,
    pub information: Option<RelayInformation>,
}

impl DbRelay {
    const FETCH_QUERY: &'static str = "SELECT * FROM relay";

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbRelay>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let output = sqlx::query_as::<_, DbRelay>(&sql).fetch_all(pool).await?;
        Ok(output)
    }

    pub async fn fetch_by_url(pool: &SqlitePool, url: &Url) -> Result<Option<DbRelay>, Error> {
        let sql = format!("{} WHERE url = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbRelay>(&sql)
            .bind(&url.to_string())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, url: &Url) -> Result<DbRelay, Error> {
        let sql = "INSERT INTO relay (url) VALUES (?)";
        sqlx::query(sql)
            .bind(&url.to_string())
            .execute(pool)
            .await?;
        let db_relay = Self::fetch_by_url(pool, url)
            .await?
            .ok_or_else(|| Error::RelayNotFound(url.to_string()))?;
        Ok(db_relay)
    }

    pub async fn update(pool: &SqlitePool, relay: &DbRelay) -> Result<(), Error> {
        let sql = "UPDATE relay SET read=?, write=?, advertise=? WHERE id=?";
        sqlx::query(sql)
            .bind(relay.read)
            .bind(relay.write)
            .bind(relay.advertise)
            .bind(relay.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, url: &Url) -> Result<(), Error> {
        let sql = "DELETE FROM relay WHERE url=?";
        sqlx::query(sql)
            .bind(&url.to_string())
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbRelay {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let url = row.try_get::<String, &str>("url")?;
        let url = url_or_err(&url, "url")?;

        Ok(DbRelay {
            id: row.try_get::<i32, &str>("id")?,
            url,
            read: row.try_get::<bool, &str>("read")?,
            write: row.try_get::<bool, &str>("write")?,
            advertise: row.try_get::<bool, &str>("advertise")?,
            information: None,
        })
    }
}
