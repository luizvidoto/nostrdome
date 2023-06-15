use directories::ProjectDirs;
use std::{fs, io::Write, path::PathBuf};
use tokio::io::AsyncWriteExt;
use toml;

use serde::{Deserialize, Serialize};

use crate::{consts::APP_PROJECT_DIRS, style::Theme};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not found project directory")]
    NotFoundProjectDirectory,

    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Deserialize Error: {0}")]
    DeserializeError(#[from] toml::de::Error),

    #[error("Serialize Error: {0}")]
    SerializeError(#[from] toml::ser::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    /// Theme of the application
    pub theme: Theme,
}

impl Config {
    fn path() -> Result<PathBuf, Error> {
        let mut path = config_dir()?;
        path.push(CONFIG_FILENAME);
        Ok(path)
    }
    pub fn load() -> Self {
        Self::load_file().unwrap_or_default()
    }
    fn load_file() -> Result<Self, Error> {
        let path = Self::path()?;
        // Check if file exists, if not create it
        if !path.exists() {
            open_and_write(&Self::default(), &path)?;
        }
        let toml = fs::read_to_string(path)?;
        let config = toml::from_str(&toml)?;
        Ok(config)
    }

    pub async fn load_file_async() -> Result<Self, Error> {
        let path = Self::path()?;
        // Check if file exists, if not create it
        if !path.exists() {
            open_and_write_async(&Self::default(), &path).await?;
        }
        let toml = tokio::fs::read_to_string(path).await?;
        let config = toml::from_str(&toml)?;
        Ok(config)
    }

    pub async fn set_theme(theme: Theme) -> Result<(), Error> {
        let mut config = Self::load_file_async().await?;
        config.theme = theme;
        config.save().await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<(), Error> {
        let config_dir = config_dir()?;

        if !config_dir.exists() {
            tracing::info!("Creating config directory: {:?}", &config_dir);
            tokio::fs::create_dir(config_dir).await?;
        };

        open_and_write_async(&self, &Self::path()?).await?;

        Ok(())
    }
}

fn open_and_write(config: &Config, path: &PathBuf) -> Result<(), Error> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;
    file.write_all(toml::to_string_pretty(config)?.as_bytes())?;
    Ok(())
}

async fn open_and_write_async(config: &Config, path: &PathBuf) -> Result<(), Error> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .await?;
    file.write_all(toml::to_string_pretty(config)?.as_bytes())
        .await?;
    Ok(())
}

fn config_dir() -> Result<PathBuf, Error> {
    let dirs = ProjectDirs::from(APP_PROJECT_DIRS.0, APP_PROJECT_DIRS.1, APP_PROJECT_DIRS.2)
        .ok_or(Error::NotFoundProjectDirectory)?;
    Ok(dirs.data_dir().into())
}

const CONFIG_FILENAME: &str = "config.toml";
