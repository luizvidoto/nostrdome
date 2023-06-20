use nostr::{Kind, Tag};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UncheckedEvent {
    pub kind: Kind,
    pub content: String,
    pub tags: Vec<Tag>,
    pub created_at: i64,
    pub pubkey: String,
    pub id: String,
    pub sig: String,
}
