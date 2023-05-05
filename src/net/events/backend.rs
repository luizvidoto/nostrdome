use nostr_sdk::Keys;
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbRelay},
    error::Error,
    net::{
        logic::{
            batch_of_events, db_add_relay, db_delete_relay, insert_pending_event, on_relay_message,
            received_event,
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
    StoreEvents((nostr_sdk::Url, Vec<nostr_sdk::Event>)),
    StoreRelayMessage((nostr_sdk::Url, nostr_sdk::RelayMessage)),
    LatestVersion(String),
    GotProfile((DbContact, nostr_sdk::Metadata)),
    Shutdown,
    None,
}

pub async fn backend_processing(pool: &SqlitePool, keys: &Keys, input: BackEndInput) -> Event {
    match input {
        BackEndInput::None => Event::None,
        // --- REQWEST ---
        BackEndInput::LatestVersion(version) => Event::LatestVersion(version),
        BackEndInput::Shutdown => Event::None,
        // --- TO DATABASE ---
        BackEndInput::DeleteRelayFromDb(db_relay) => {
            process_async_with_event(db_delete_relay(&pool, db_relay)).await
        }
        BackEndInput::GotProfile((mut db_contact, metadata)) => {
            db_contact = db_contact.with_profile_meta(&metadata);
            match DbContact::update(&pool, &db_contact).await {
                Ok(_) => Event::ContactUpdated(db_contact.clone()),
                Err(e) => Event::Error(e.to_string()),
            }
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
            process_async_with_event(received_event(&pool, &keys, nostr_event, &relay_url)).await
        }
        BackEndInput::StoreEvents((relay_url, events)) => {
            process_async_with_event(batch_of_events(&pool, &keys, events, &relay_url)).await
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
