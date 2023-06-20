// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Metadata

use core::fmt;

use serde::{Deserialize, Serialize};
use url::Url;

/// [`Metadata`] error
#[derive(Debug)]
pub enum Error {
    /// Error serializing or deserializing JSON data
    Json(serde_json::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "json error: {e}"),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// ChannelMetadata
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChannelMetadata {
    /// Name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Picture url
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

impl Default for ChannelMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelMetadata {
    /// New empty [`ChannelMetadata`]
    pub fn new() -> Self {
        Self {
            name: None,
            about: None,
            picture: None,
        }
    }

    /// Deserialize [`ChannelMetadata`] from `JSON` string
    pub fn from_json<S>(json: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Ok(serde_json::from_str(&json.into())?)
    }

    /// Serialize [`ChannelMetadata`] to `JSON` string
    pub fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }

    /// Set name
    pub fn name<S>(self, name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    /// Set about
    pub fn about<S>(self, about: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            about: Some(about.into()),
            ..self
        }
    }

    /// Set picture
    pub fn picture(self, url: Url) -> Self {
        Self {
            picture: Some(url.into()),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_metadata() {
        let content = r#"{"name":"myname","about":"Description"}"#;
        let metadata = ChannelMetadata::from_json(content).unwrap();
        assert_eq!(
            metadata,
            ChannelMetadata::new().name("myname").about("Description")
        );

        let content = r#"{"name":"myname","about":"Description","picture":"https://some-picture.com/200/300"}"#;
        let metadata = ChannelMetadata::from_json(content).unwrap();
        assert_eq!(
            metadata,
            ChannelMetadata::new()
                .name("myname")
                .about("Description")
                .picture(Url::parse("https://some-picture.com/200/300").unwrap())
        );
    }
}
