// use crate::error::Error;

use chrono::{NaiveDateTime, Utc};
use nostr_sdk::{RelayStatus, Url};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{
    error::Error,
    utils::{millis_to_naive_or_err, url_or_err},
};

use super::UserConfig;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbRelayStatus(pub RelayStatus);
impl DbRelayStatus {
    pub fn new(relay_status: RelayStatus) -> Self {
        Self(relay_status)
    }
    pub fn as_u8(&self) -> u8 {
        match self.0 {
            RelayStatus::Initialized => 0,
            RelayStatus::Connected => 1,
            RelayStatus::Connecting => 2,
            RelayStatus::Disconnected => 3,
            RelayStatus::Terminated => 4,
        }
    }
}
impl From<u8> for DbRelayStatus {
    fn from(value: u8) -> Self {
        let inside = match value {
            0 => RelayStatus::Initialized,
            1 => RelayStatus::Connected,
            2 => RelayStatus::Connecting,
            3 => RelayStatus::Disconnected,
            4..=u8::MAX => RelayStatus::Terminated,
        };
        DbRelayStatus(inside)
    }
}
impl std::fmt::Display for DbRelayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Debug, Clone)]
pub struct DbRelay {
    pub url: Url,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub read: bool,
    pub write: bool,
    pub advertise: bool,
    pub last_connected_at: Option<NaiveDateTime>,
    pub status: Option<DbRelayStatus>,
    pub have_error: Option<String>,
}

impl DbRelay {
    const FETCH_QUERY: &'static str =
        "SELECT url, read, write, advertise, created_at, updated_at, have_error FROM relay";

    pub fn with_status(mut self, status: RelayStatus) -> Self {
        self.status = Some(DbRelayStatus(status));
        self
    }
    pub fn with_last_connected_at(mut self, last_connected_at: NaiveDateTime) -> Self {
        self.last_connected_at = Some(last_connected_at);
        self
    }

    pub fn new(url: Url) -> DbRelay {
        DbRelay {
            url,
            read: true,
            write: true,
            advertise: false,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            last_connected_at: None,
            status: None,
            have_error: None,
        }
    }

    pub fn from_str(url: &str) -> Result<Self, Error> {
        let url = Url::parse(url)?;
        Ok(Self::new(url))
    }

    pub async fn fetch(pool: &SqlitePool) -> Result<Vec<DbRelay>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let output = sqlx::query_as::<_, DbRelay>(&sql).fetch_all(pool).await?;
        Ok(output)
    }

    pub async fn fetch_one(pool: &SqlitePool, url: &Url) -> Result<Option<DbRelay>, Error> {
        let sql = format!("{} WHERE url = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbRelay>(&sql)
            .bind(&url.to_string())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, db_relay: &DbRelay) -> Result<(), Error> {
        let sql = "INSERT INTO relay (url, read, write, advertise) \
             VALUES (?1, ?2, ?3, ?4)";

        sqlx::query(sql)
            .bind(&db_relay.url.to_string())
            .bind(&db_relay.read)
            .bind(&db_relay.write)
            .bind(&db_relay.advertise)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, relay: &DbRelay) -> Result<(), Error> {
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());

        let sql = "UPDATE relay SET read=?, write=?, advertise=?, updated_at=? WHERE url=?";

        sqlx::query(sql)
            .bind(&relay.read)
            .bind(&relay.write)
            .bind(&relay.advertise)
            .bind(now_utc.timestamp_millis())
            .bind(&relay.url.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, db_relay: &DbRelay) -> Result<(), Error> {
        let sql = "DELETE FROM relay WHERE url=?";

        sqlx::query(sql)
            .bind(&db_relay.url.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub(crate) async fn update_with_error(
        pool: &SqlitePool,
        url: &Url,
        message: &str,
    ) -> Result<DbRelay, Error> {
        let mut db_relay = DbRelay::fetch_one(pool, url)
            .await?
            .ok_or_else(|| Error::RelayNotFound(url.to_string()))?;
        let now_utc = UserConfig::get_corrected_time(pool)
            .await
            .unwrap_or(Utc::now().naive_utc());
        let sql = "UPDATE relay SET updated_at=?, have_error=? WHERE url=?";

        sqlx::query(sql)
            .bind(now_utc.timestamp_millis())
            .bind(message)
            .bind(&db_relay.url.to_string())
            .execute(pool)
            .await?;

        db_relay.have_error = Some(message.to_owned());
        db_relay.updated_at = now_utc;

        Ok(db_relay)
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbRelay {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let created_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("created_at")?, "created_at")?;
        let updated_at =
            millis_to_naive_or_err(row.try_get::<i64, &str>("updated_at")?, "updated_at")?;
        let url = row.try_get::<String, &str>("url")?;
        let url = url_or_err(&url, "url")?;

        let have_error: Option<String> = row.get("have_error");
        Ok(DbRelay {
            url,
            created_at,
            updated_at,
            read: row.try_get::<bool, &str>("read")?,
            write: row.try_get::<bool, &str>("write")?,
            advertise: row.try_get::<bool, &str>("advertise")?,
            last_connected_at: None,
            status: None,
            have_error,
        })
    }
}
