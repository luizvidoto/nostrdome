use directories::ProjectDirs;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::error::Error;

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

            s.initial_setup().await?;
            s.check_and_upgrade().await?;

            Ok(s)
        } else {
            Err(Error::DatabaseSetup("Not found project directory".into()))
        }
    }

    async fn initial_setup(&self) -> Result<(), Error> {
        tracing::warn!("Database initial setup");
        for sql in INITIAL_SETUP {
            sqlx::query(sql).execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn check_and_upgrade(&self) -> Result<(), Error> {
        let version = sqlx::query("SELECT schema_version FROM local_settings LIMIT 1")
            .map(|r: SqliteRow| r.get(0))
            .fetch_one(&self.pool)
            .await?;

        tracing::warn!("DB version: {}", version);
        self.upgrade(version).await?;
        Ok(())
    }

    async fn upgrade(&self, mut version: i32) -> Result<(), Error> {
        if version > UPGRADE_SQL.len() as i32 {
            panic!(
                "Database version {} is newer than this binary which expects version {}.",
                version,
                UPGRADE_SQL.len()
            );
        }

        while version < UPGRADE_SQL.len() as i32 {
            tracing::warn!("Upgrading database to version {}", version + 1);

            sqlx::query(UPGRADE_SQL[version as usize])
                .execute(&self.pool)
                .await?;

            version += 1;

            sqlx::query("UPDATE local_settings SET schema_version = ?")
                .bind(version)
                .execute(&self.pool)
                .await?;
        }

        tracing::warn!("Database is at version {}", version);

        Ok(())
    }
}

const INITIAL_SETUP: [&str; 3] = [
    include_str!("../../migrations/1_setup.sql"),
    include_str!("../../migrations/2_event.sql"),
    include_str!("../../migrations/3_relay.sql"),
];

const UPGRADE_SQL: [&str; 1] = [include_str!("../../migrations/fake_mig.sql")];
