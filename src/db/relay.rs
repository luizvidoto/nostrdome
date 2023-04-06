// use crate::error::Error;

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Error, types::RelayUrl};

#[derive(Debug, Clone)]
pub struct DbRelay {
    pub url: RelayUrl,
    pub last_connected_at: Option<i32>,
    pub read: bool,
    pub write: bool,
    pub advertise: bool,
}

impl DbRelay {
    pub fn new(url: RelayUrl) -> DbRelay {
        DbRelay {
            url,
            last_connected_at: None,
            read: false,
            write: false,
            advertise: false,
        }
    }

    const FETCH_QUERY: &'static str =
        "SELECT url, last_connected_at, read, write, advertise FROM relay";

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbRelay>, Error> {
        let sql = Self::FETCH_QUERY.to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let output = sqlx::query_as::<_, DbRelay>(&sql).fetch_all(pool).await?;

        Ok(output)
    }

    pub async fn fetch_one(pool: &SqlitePool, url: &RelayUrl) -> Result<Option<DbRelay>, Error> {
        let sql = format!("{} WHERE url = ?", Self::FETCH_QUERY);
        Ok(sqlx::query_as::<_, DbRelay>(&sql)
            .bind(url.0.as_str())
            .fetch_optional(pool)
            .await?)
    }

    pub async fn insert(pool: &SqlitePool, relay: DbRelay) -> Result<(), Error> {
        let sql = "INSERT OR IGNORE INTO relay (url, last_connected_at, \
                   read, write, advertise) \
             VALUES (?1, ?2, ?3, ?4, ?5)";

        sqlx::query(sql)
            .bind(&relay.url.0)
            .bind(&relay.last_connected_at)
            .bind(&relay.read)
            .bind(&relay.write)
            .bind(&relay.advertise)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update(pool: &SqlitePool, relay: DbRelay) -> Result<(), Error> {
        let sql = "UPDATE relay SET last_connected_at=?, read=?, write=?, advertise=? WHERE url=?";

        sqlx::query(sql)
            .bind(&relay.last_connected_at)
            .bind(&relay.read)
            .bind(&relay.write)
            .bind(&relay.advertise)
            .bind(&relay.url.0)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, relay_url: RelayUrl) -> Result<(), Error> {
        let sql = "DELETE FROM relay WHERE url=?";

        sqlx::query(sql).bind(&relay_url.0).execute(pool).await?;

        Ok(())
    }
}

impl sqlx::FromRow<'_, SqliteRow> for DbRelay {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let url = row.try_get::<String, &str>("url")?;
        let url = RelayUrl::try_from_str(url.as_str()).map_err(|e| sqlx::Error::ColumnDecode {
            index: "url".into(),
            source: Box::new(e),
        })?;
        Ok(DbRelay {
            url,
            last_connected_at: row.try_get::<Option<i32>, &str>("last_connected_at")?,
            read: row.try_get::<bool, &str>("read")?,
            write: row.try_get::<bool, &str>("write")?,
            advertise: row.try_get::<bool, &str>("advertise")?,
        })
    }
}
