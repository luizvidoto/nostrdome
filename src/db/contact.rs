use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbContact {
    pub pubkey: String,
    pub recommended_relay: Option<String>,
    pub petname: Option<String>,
    pub profile_image: Option<String>,
}
