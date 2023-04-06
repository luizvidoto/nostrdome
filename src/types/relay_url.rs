use crate::error::Error;
use regex::Regex;
use std::fmt;

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
        match Self::is_url_valid(url) {
            true => {
                let mut url_lower = url.to_lowercase();
                if !url_lower.ends_with('/') {
                    url_lower.push('/');
                }
                Ok(RelayUrl(url_lower))
            }
            false => Err(Error::InvalidUrlRegex(url.to_string())),
        }
    }

    /// As &str
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
    use std::str::FromStr;

    use super::*;
    use url::Url;

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
        let url = Url::from_str("Wss://MyRelay.example.COM/PATH?Query").unwrap();
        assert_eq!(url.as_str(), "wss://myrelay.example.com/PATH?Query");
    }

    #[test]
    fn test_relay_url_slash() {
        let input = "Wss://MyRelay.example.COM";
        let url = RelayUrl::try_from_str(input).unwrap();
        assert_eq!(url.as_str(), "wss://myrelay.example.com/");
    }
}
