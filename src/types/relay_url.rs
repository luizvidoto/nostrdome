use nostr::prelude::UncheckedUrl;
use regex::Regex;
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid URL regex: \"{0}\"")]
    InvalidUrlRegex(String),

    #[error("Invalid relay URL: \"{0}\"")]
    InvalidRelayUrl(String),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct RelayUrl(pub String);

impl fmt::Display for RelayUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RelayUrl {
    const URL_PATTERN: &'static str = r"(?i)^(wss?://)(([a-z0-9-]+\.)+[a-z]{2,}|([0-9]{1,3}\.){3}[0-9]{1,3})(:[0-9]+)?(/[\w/?=-]*)?$";
    pub fn is_url_valid(url: &str) -> bool {
        let re = Regex::new(Self::URL_PATTERN).unwrap();
        re.is_match(url)
    }
    pub fn try_from_str(url: &str) -> Result<RelayUrl, Error> {
        if Self::is_url_valid(url) {
            let mut url_lower = url.to_lowercase();
            if !url_lower.ends_with('/') {
                url_lower.push('/');
            }
            Ok(RelayUrl(url_lower))
        } else {
            Err(Error::InvalidUrlRegex(url.to_string()))
        }
    }

    pub fn try_into_unchecked_url(url: &str) -> Result<UncheckedUrl, Error> {
        if let Ok(_) = Self::try_from_str(url) {
            UncheckedUrl::from_str(url).map_err(|_| Error::InvalidRelayUrl(url.to_string()))
        } else {
            Err(Error::InvalidRelayUrl(url.to_string()))
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    // Mock data for testing
    #[allow(dead_code)]
    pub(crate) fn mock() -> String {
        "wss://example.com".to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn regex_test() {
        let urls = vec![
            "wss://example.COM/PATH?Query",
            "ws://example.whatever/",
            "Wss://192.168.0.1:8080/",
            "ws://192.168.0.1:8080/",
            "wSs://myserver.example.net/",
            "ws://My-seRvEr.example.IO/PATH?Query",
        ];

        for url in urls {
            assert!(RelayUrl::is_url_valid(url), "'{}' not valid", url);
        }
    }

    #[test]
    fn test_url_case() {
        let url = RelayUrl::try_from_str("Wss://MyRelay.example.COM/PATH?Query").unwrap();
        assert_eq!(url.as_str(), "wss://myrelay.example.com/PATH?Query");
    }

    #[test]
    fn test_relay_url_slash() {
        let input = "Wss://MyRelay.example.COM";
        let url = RelayUrl::try_from_str(input).unwrap();
        assert_eq!(url.as_str(), "wss://myrelay.example.com/");
    }
}
