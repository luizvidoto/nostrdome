use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DbContact {
    pub source: String,
    pub contact: String,
    pub relay: Option<String>,
    pub petname: Option<String>,
}
