// use crate::error::Error;

use super::relay_url::RelayUrl;

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
}
