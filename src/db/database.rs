use crate::error::Error;
use directories::ProjectDirs;
use sqlx::SqlitePool;
use std::cmp::Ordering;

/// Latest database version
pub const DB_VERSION: usize = 1;

/// Startup DB Pragmas
pub const STARTUP_SQL: &str = r##"
PRAGMA main.synchronous=NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA journal_size_limit=32768;
pragma mmap_size = 17179869184; -- cap mmap at 16GB
"##;

const INITIAL_SETUP: [&str; 6] = [
    include_str!("../../migrations/1_setup.sql"),
    include_str!("../../migrations/2_event.sql"),
    include_str!("../../migrations/3_relay.sql"),
    include_str!("../../migrations/4_tag.sql"),
    include_str!("../../migrations/5_contact.sql"),
    include_str!("../../migrations/6_message.sql"),
];

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(in_memory: bool) -> Result<Self, Error> {
        if let Some(dirs) = ProjectDirs::from("com", "NostrDome", "NostrDome") {
            tracing::warn!("Creating project directory");
            let project_dir = dirs.config_dir();
            std::fs::create_dir_all(project_dir)?;

            tracing::warn!("Creating database");
            let db_url = if in_memory {
                "sqlite::memory:".to_owned()
            } else {
                let mut path_ext = String::new();
                for dir in project_dir.iter() {
                    if let Some(p) = dir.to_str() {
                        if p.contains("\\") || p.contains("/") {
                            continue;
                        }
                        path_ext.push_str(&format!("/{}", p));
                    }
                }
                format!("sqlite://{}/nostrdome.db3?mode=rwc", &path_ext)
            };

            tracing::warn!("Connecting database");
            let pool = SqlitePool::connect(&db_url).await?;

            let s = Self { pool };

            upgrade_db(&s.pool).await?;

            Ok(s)
        } else {
            Err(Error::DatabaseSetup("Not found project directory".into()))
        }
    }
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
    sqlx::query(STARTUP_SQL).execute(pool).await?;
    tracing::debug!("SQLite PRAGMA startup completed");
    Ok(())
}

/// Determine the current application database schema version.
pub async fn curr_db_version(pool: &SqlitePool) -> Result<usize, Error> {
    let query = "PRAGMA user_version;";
    let curr_version: u32 = sqlx::query_scalar(query).fetch_one(pool).await?;
    Ok(curr_version as usize)
}

async fn initial_setup(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    tracing::warn!("Database initial setup");
    for sql in INITIAL_SETUP {
        sqlx::query(sql).execute(pool).await?;
    }
    log::info!("Database schema initialized to v1");
    Ok(1)
}

const _UPGRADE_SQL: [&str; 0] = [
// include_str!("../../migrations/migration.sql")
];

/* async fn mig_1_to_2(pool: &SqlitePool) -> Result<usize, Error> {
    sqlx::query(include_str!("../migrations/002.sql")).execute(pool).await?;
    log::info!("database schema upgraded v1 -> v2");
    Ok(2)
} */
