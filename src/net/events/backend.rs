use nostr_sdk::Keys;
use sqlx::SqlitePool;

use crate::{
    db::DbRelay,
    net::{
        logic::{
            db_add_relay, db_delete_relay, insert_pending_event, on_relay_message, received_event,
        },
        process_async_fn, process_async_with_event,
    },
};

use super::Event;

#[derive(Debug, Clone)]
pub enum BackEndInput {
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    StorePendingEvent(nostr_sdk::Event),
    StoreEvent((nostr_sdk::Url, nostr_sdk::Event)),
    StoreRelayMessage((nostr_sdk::Url, nostr_sdk::RelayMessage)),
    Shutdown,
}

pub async fn backend_processing(pool: &SqlitePool, keys: &Keys, input: BackEndInput) -> Event {
    match input {
        BackEndInput::Shutdown => Event::None,
        // --- TO DATABASE ---
        BackEndInput::DeleteRelayFromDb(db_relay) => {
            process_async_with_event(db_delete_relay(&pool, db_relay)).await
        }
        BackEndInput::ToggleRelayRead((mut db_relay, read)) => {
            db_relay.read = read;
            match DbRelay::update(&pool, &db_relay).await {
                Ok(_) => Event::RelayUpdated(db_relay.clone()),
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::ToggleRelayWrite((mut db_relay, write)) => {
            db_relay.write = write;
            match DbRelay::update(&pool, &db_relay).await {
                Ok(_) => Event::RelayUpdated(db_relay.clone()),
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::AddRelayToDb(db_relay) => {
            process_async_with_event(db_add_relay(&pool, db_relay)).await
        }
        BackEndInput::StorePendingEvent(nostr_event) => {
            process_async_fn(insert_pending_event(&pool, &keys, nostr_event), |event| {
                event
            })
            .await
        }
        BackEndInput::StoreEvent((relay_url, nostr_event)) => {
            process_async_fn(
                received_event(&pool, &keys, nostr_event, &relay_url),
                |event| event,
            )
            .await
        }
        BackEndInput::StoreRelayMessage((relay_url, relay_message)) => {
            process_async_fn(
                on_relay_message(&pool, &relay_url, &relay_message),
                |event| event,
            )
            .await
        }
    }
}
