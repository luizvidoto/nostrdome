use crate::{consts::APP_PROJECT_DIRS, db::UserConfig};
use thiserror::Error;

use directories::ProjectDirs;
use sqlx::SqlitePool;
use std::cmp::Ordering;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not found project directory")]
    NotFoundProjectDirectory,

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error(
        "Database version is newer than supported by this executable (v{current} > v{db_ver})"
    )]
    NewerDbVersion { current: usize, db_ver: usize },
}

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: SqlitePool,
    pub cache_pool: SqlitePool,
}

impl Database {
    pub async fn new(pubkey: &str) -> Result<Self, Error> {
        let pool = db_pool(pubkey).await?;
        let cache_pool = get_cache_pool().await?;
        let s = Self { pool, cache_pool };
        Ok(s)
    }
}

async fn db_pool(pubkey: &str) -> Result<SqlitePool, Error> {
    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let db_url = if IN_MEMORY {
        "sqlite::memory:".to_owned()
    } else {
        let mut path_ext = String::new();
        for dir in data_dir.iter() {
            if let Some(p) = dir.to_str() {
                if p.contains("\\") || p.contains("/") {
                    continue;
                }
                path_ext.push_str(&format!("/{}", p));
            }
        }
        format!("sqlite://{}/{}.db3?mode=rwc", &path_ext, pubkey)
    };

    tracing::info!("Connecting database");
    let pool = SqlitePool::connect(&db_url).await?;
    upgrade_db(&pool).await?;
    Ok(pool)
}

async fn get_cache_pool() -> Result<SqlitePool, Error> {
    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    let cache_dir = dirs.cache_dir();
    std::fs::create_dir_all(cache_dir)?;

    let db_url = if IN_MEMORY {
        "sqlite::memory:".to_owned()
    } else {
        let mut path_ext = String::new();
        for dir in cache_dir.iter() {
            if let Some(p) = dir.to_str() {
                if p.contains("\\") || p.contains("/") {
                    continue;
                }
                path_ext.push_str(&format!("/{}", p));
            }
        }
        format!("sqlite://{}.db3?mode=rwc", &path_ext)
    };

    tracing::info!("Connecting to cache database");
    let cache_pool = SqlitePool::connect(&db_url).await?;

    tracing::info!("Cache setup");

    upgrade_cache_db(&cache_pool).await?;

    Ok(cache_pool)
}

pub async fn upgrade_cache_db(cache_pool: &SqlitePool) -> Result<(), Error> {
    for sql in CACHE_SETUP {
        sqlx::query(sql).execute(cache_pool).await?;
    }
    Ok(())
}

/// Upgrade DB to latest version, and execute pragma settings
pub async fn upgrade_db(pool: &SqlitePool) -> Result<(), Error> {
    // check the version.
    let mut curr_version = curr_db_version(pool).await?;
    tracing::info!("DB version = {:?}", curr_version);

    match curr_version.cmp(&DB_VERSION) {
        // Database is new or not current
        Ordering::Less => {
            // initialize from scratch
            if curr_version == 0 {
                curr_version = initial_setup(pool).await?;
            }

            // for initialized but out-of-date schemas, proceed to
            // upgrade sequentially until we are current.
            /* if curr_version == 1 {
                curr_version = mig_1_to_2(pool).await?;
            } */
            /* if curr_version == 2 {
                curr_version = mig_2_to_3(pool).await?;
            } */

            if curr_version == DB_VERSION {
                tracing::info!("All migration scripts completed successfully (v{DB_VERSION})");
            }
        }
        // Database is current, all is good
        Ordering::Equal => {
            tracing::debug!("Database version was already current (v{DB_VERSION})");
        }
        // Database is newer than what this code understands, abort
        Ordering::Greater => {
            return Err(Error::NewerDbVersion {
                current: curr_version,
                db_ver: DB_VERSION,
            });
        }
    }

    // Setup PRAGMA
    // sqlx::query(STARTUP_SQL).execute(pool).await?;
    // tracing::debug!("SQLite PRAGMA startup completed");
    Ok(())
}

/// Determine the current application database schema version.
pub async fn curr_db_version(pool: &SqlitePool) -> Result<usize, Error> {
    let query = "PRAGMA user_version;";
    let curr_version: u32 = sqlx::query_scalar(query).fetch_one(pool).await?;
    Ok(curr_version as usize)
}

async fn initial_setup(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    tracing::info!("Database initial setup");

    for sql in INITIAL_SETUP {
        sqlx::query(sql).execute(pool).await?;
    }

    UserConfig::setup_user_config(pool).await?;

    tracing::info!("Database schema initialized to v1");

    Ok(1)
}

const _UPGRADE_SQL: [&str; 0] = [
// include_str!("../../migrations/migration.sql")
];

/* async fn mig_1_to_2(pool: &SqlitePool) -> Result<usize, Error> {
    sqlx::query(include_str!("../migrations/002.sql")).execute(pool).await?;
    tracing::info!("database schema upgraded v1 -> v2");
    Ok(2)
} */

/// Latest database version
pub const DB_VERSION: usize = 1;

const INITIAL_SETUP: [&str; 9] = [
    include_str!("../../migrations/1_setup.sql"),
    include_str!("../../migrations/2_event.sql"),
    include_str!("../../migrations/3_relay.sql"),
    include_str!("../../migrations/5_contact.sql"),
    include_str!("../../migrations/6_message.sql"),
    include_str!("../../migrations/7_user_config.sql"),
    include_str!("../../migrations/8_relay_response.sql"),
    include_str!("../../migrations/9_channel_message.sql"),
    include_str!("../../migrations/10_subscribed_channel.sql"),
];

const CACHE_SETUP: [&str; 5] = [
    include_str!("../../migrations/cache/1_setup.sql"),
    include_str!("../../migrations/cache/2_profile_meta_cache.sql"),
    include_str!("../../migrations/cache/3_channel_cache.sql"),
    include_str!("../../migrations/cache/4_image_cache.sql"),
    include_str!("../../migrations/cache/5_channel_member_map.sql"),
];

const IN_MEMORY: bool = false;
