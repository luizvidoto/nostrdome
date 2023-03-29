// use crate::error::Error;

#[derive(Debug, Clone)]
pub struct DbRelay {
    pub url: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub rank: u64,
    pub last_connected_at: Option<u64>,
    pub last_general_eose_at: Option<u64>,
    pub read: bool,
    pub write: bool,
    pub advertise: bool,
}

impl DbRelay {
    pub fn new(url: impl Into<String>) -> DbRelay {
        DbRelay {
            url: url.into(),
            success_count: 0,
            failure_count: 0,
            rank: 3,
            last_connected_at: None,
            last_general_eose_at: None,
            read: false,
            write: false,
            advertise: false,
        }
    }
}
