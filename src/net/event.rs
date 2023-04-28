use crate::db::{store_last_event_timestamp, DbEvent};
use crate::error::Error;

use nostr_sdk::{Keys, Kind, Url};
use sqlx::SqlitePool;

use super::Event;
