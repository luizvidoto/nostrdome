// use crate::error::Error;

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Error, types::RelayUrl};

#[derive(Debug, Clone)]
pub struct DbRelay {
    pub url: RelayUrl,
    pub last_connected_at: Option<u64>,
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

    pub async fn fetch(pool: &SqlitePool, criteria: Option<&str>) -> Result<Vec<DbRelay>, Error> {
        let sql = "SELECT url, last_connected_at, read, write, advertise FROM relay".to_owned();
        let sql = match criteria {
            None => sql,
            Some(crit) => format!("{} WHERE {}", sql, crit),
        };
        let output = sqlx::query_as::<_, DbRelay>(&sql).fetch_all(pool).await?;

        Ok(output)

        // let output: Result<Vec<DbRelay>, Error> = spawn_blocking(move || {
        //     let maybe_db = GLOBALS.db.blocking_lock();
        //     let db = maybe_db.as_ref().unwrap();

        //     let mut stmt = db.prepare(&sql)?;
        //     let mut rows = stmt.query([])?;
        //     let mut output: Vec<DbRelay> = Vec::new();
        //     while let Some(row) = rows.next()? {
        //         let s: String = row.get(0)?;
        //         // just skip over invalid relay URLs
        //         if let Ok(url) = RelayUrl::try_from_str(&s) {
        //             output.push(DbRelay {
        //                 url,
        //                 success_count: row.get(1)?,
        //                 failure_count: row.get(2)?,
        //                 rank: row.get(3)?,
        //                 last_connected_at: row.get(4)?,
        //                 last_general_eose_at: row.get(5)?,
        //                 read: row.get(6)?,
        //                 write: row.get(7)?,
        //                 advertise: row.get(8)?,
        //             });
        //         }
        //     }
        //     Ok::<Vec<DbRelay>, Error>(output)
        // })
        // .await?;

        // output
    }

    // pub async fn fetch_one(url: &RelayUrl) -> Result<Option<DbRelay>, Error> {
    //     let relays = DbRelay::fetch(Some(&format!("url='{}'", url.0))).await?;

    //     if relays.is_empty() {
    //         Ok(None)
    //     } else {
    //         Ok(Some(relays[0].clone()))
    //     }
    // }

    // pub async fn insert(relay: DbRelay) -> Result<(), Error> {
    //     let sql = "INSERT OR IGNORE INTO relay (url, success_count, failure_count, rank, \
    //                last_connected_at, last_general_eose_at, read, write, advertise) \
    //          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";

    //     spawn_blocking(move || {
    //         let maybe_db = GLOBALS.db.blocking_lock();
    //         let db = maybe_db.as_ref().unwrap();

    //         let mut stmt = db.prepare(sql)?;
    //         stmt.execute((
    //             &relay.url.0,
    //             &relay.success_count,
    //             &relay.failure_count,
    //             &relay.rank,
    //             &relay.last_connected_at,
    //             &relay.last_general_eose_at,
    //             &relay.read,
    //             &relay.write,
    //             &relay.advertise,
    //         ))?;
    //         Ok::<(), Error>(())
    //     })
    //     .await??;

    //     Ok(())
    // }

    // pub async fn update(relay: DbRelay) -> Result<(), Error> {
    //     let sql = "UPDATE relay SET success_count=?, failure_count=?, rank=?, \
    //                last_connected_at=?, last_general_eose_at=?, read=?, write=?, advertise=? WHERE url=?";

    //     spawn_blocking(move || {
    //         let maybe_db = GLOBALS.db.blocking_lock();
    //         let db = maybe_db.as_ref().unwrap();

    //         let mut stmt = db.prepare(sql)?;
    //         stmt.execute((
    //             &relay.success_count,
    //             &relay.failure_count,
    //             &relay.rank,
    //             &relay.last_connected_at,
    //             &relay.last_general_eose_at,
    //             &relay.read,
    //             &relay.write,
    //             &relay.advertise,
    //             &relay.url.0,
    //         ))?;
    //         Ok::<(), Error>(())
    //     })
    //     .await??;

    //     Ok(())
    // }
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
            last_connected_at: row
                .try_get::<Option<i32>, &str>("last_connected_at")?
                .map(|n| n as u64),
            read: row.try_get::<bool, &str>("read")?,
            write: row.try_get::<bool, &str>("write")?,
            advertise: row.try_get::<bool, &str>("advertise")?,
        })
    }
}
