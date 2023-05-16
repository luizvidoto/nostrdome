use std::path::PathBuf;

use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, ProfileCache, UserConfig},
    error::Error,
    net::{
        events::nostr::NostrInput,
        operations::event::{
            insert_confirmed_event, insert_pending_contact_list, insert_pending_dm,
            insert_pending_metadata, on_relay_message,
        },
        process_async_with_event, ImageKind,
    },
    types::ChatMessage,
};

use super::Event;

#[derive(Debug, Clone)]
pub enum BackEndInput {
    NtpTime(u64),
    RelayConfirmation {
        relay_response: DbRelayResponse,
        db_event: DbEvent,
    },
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    StorePendingContactList((nostr_sdk::Event, Vec<DbContact>)),
    StorePendingMetadata((nostr_sdk::Event, nostr_sdk::Metadata)),
    StorePendingMessage {
        ns_event: nostr_sdk::Event,
        db_contact: DbContact,
        content: String,
    },
    StoreConfirmedEvent((nostr_sdk::Url, nostr_sdk::Event)),
    StoreRelayMessage((nostr_sdk::Url, nostr_sdk::RelayMessage)),
    LatestVersion(String),
    ImageDownloaded {
        kind: ImageKind,
        public_key: XOnlyPublicKey,
        path: PathBuf,
    },
    Shutdown,
    FinishedPreparingNostr,
    Error(String),
    Ok(Event),
    FailedToSendEvent {
        relay_url: nostr_sdk::Url,
        event_hash: nostr_sdk::EventId,
        status: bool,
        message: String,
    },
}

pub async fn backend_processing(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    input: BackEndInput,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Event {
    match input {
        BackEndInput::Ok(event) => event,
        BackEndInput::Error(e) => Event::Error(e),
        // --- REQWEST ---
        BackEndInput::LatestVersion(version) => Event::LatestVersion(version),
        BackEndInput::Shutdown => Event::None,
        // --- TO DATABASE ---
        BackEndInput::NtpTime(total_microseconds) => {
            tracing::info!("NTP time: {}", total_microseconds);
            match UserConfig::update_ntp_offset(pool, total_microseconds).await {
                Ok(_) => Event::SyncedWithNtpServer,
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::FinishedPreparingNostr => Event::FinishedPreparing,
        BackEndInput::ImageDownloaded {
            kind,
            public_key,
            path,
        } => {
            process_async_with_event(update_profile_cache(
                pool, cache_pool, keys, public_key, kind, path,
            ))
            .await
        }
        BackEndInput::DeleteRelayFromDb(db_relay) => match DbRelay::delete(pool, &db_relay).await {
            Ok(_) => Event::RelayDeleted(db_relay),
            Err(e) => Event::Error(e.to_string()),
        },
        BackEndInput::StorePendingMessage {
            ns_event,
            db_contact,
            content,
        } => {
            process_async_with_event(insert_pending_dm(
                pool,
                keys,
                ns_event,
                db_contact,
                content,
                nostr_sender,
            ))
            .await
        }
        BackEndInput::StorePendingContactList((ns_event, contact_list)) => {
            process_async_with_event(insert_pending_contact_list(
                pool,
                ns_event,
                contact_list,
                nostr_sender,
            ))
            .await
        }
        BackEndInput::StorePendingMetadata((ns_event, metadata)) => {
            process_async_with_event(insert_pending_metadata(
                pool,
                ns_event,
                metadata,
                nostr_sender,
            ))
            .await
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
        BackEndInput::AddRelayToDb(db_relay) => match DbRelay::insert(pool, &db_relay).await {
            Ok(_) => Event::RelayCreated(db_relay),
            Err(e) => Event::Error(e.to_string()),
        },
        BackEndInput::StoreConfirmedEvent((relay_url, ns_event)) => {
            process_async_with_event(insert_confirmed_event(
                pool,
                cache_pool,
                keys,
                back_sender,
                nostr_sender,
                &relay_url,
                ns_event,
            ))
            .await
        }
        BackEndInput::RelayConfirmation { db_event, .. } => {
            process_async_with_event(on_event_confirmation(pool, cache_pool, keys, &db_event)).await
        }
        BackEndInput::FailedToSendEvent {
            message, relay_url, ..
        } => {
            tracing::info!("Relay {} - Not ok: {}", relay_url, &message);
            Event::None
        }
        BackEndInput::StoreRelayMessage((relay_url, relay_message)) => {
            match on_relay_message(&pool, &relay_url, &relay_message).await {
                Ok(input) => {
                    if let Err(e) = back_sender.try_send(input) {
                        tracing::error!("Error sending to back_sender: {}", e);
                    }
                    Event::None
                }
                Err(e) => Event::Error(e.to_string()),
            }
        }
    }
}

pub async fn handle_recommend_relay(ns_event: nostr_sdk::Event) -> Result<Event, Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&ns_event);
    Ok(Event::None)
}

pub async fn on_event_confirmation(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<Event, Error> {
    match db_event.kind {
        nostr_sdk::Kind::ContactList => Ok(Event::ConfirmedContactList(db_event.to_owned())),
        nostr_sdk::Kind::Metadata => Ok(Event::ConfirmedMetadata(db_event.to_owned())),
        nostr_sdk::Kind::EncryptedDirectMessage => {
            handle_dm_confirmation(pool, cache_pool, keys, db_event).await
        }
        _ => Ok(Event::None),
    }
}

async fn handle_dm_confirmation(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<Event, Error> {
    let relay_url = db_event
        .relay_url
        .as_ref()
        .ok_or(Error::NotConfirmedEvent)?;
    if let Some(db_message) = DbMessage::fetch_one(pool, db_event.event_id()?).await? {
        let db_message = DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
        if let Some(db_contact) =
            DbContact::fetch_one(pool, cache_pool, &db_message.contact_chat()).await?
        {
            let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
            return Ok(Event::ConfirmedDM((db_contact, chat_message)));
        } else {
            tracing::error!("No contact found for confirmed message");
        }
    } else {
        tracing::error!("No message found for confirmation event");
    }
    Ok(Event::None)
}

pub async fn update_profile_cache(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    public_key: XOnlyPublicKey,
    kind: ImageKind,
    path: PathBuf,
) -> Result<Event, Error> {
    let _ = ProfileCache::update_local_path(cache_pool, &public_key, kind, &path).await?;

    let event = match keys.public_key() == public_key {
        true => match kind {
            ImageKind::Profile => Event::UserProfilePictureUpdated(path),
            ImageKind::Banner => Event::UserBannerPictureUpdated(path),
        },
        false => {
            if let Some(db_contact) = DbContact::fetch_one(pool, cache_pool, &public_key).await? {
                Event::ContactUpdated(db_contact)
            } else {
                tracing::warn!(
                    "Image downloaded for contact not found in database: {}",
                    public_key
                );
                Event::None
            }
        }
    };
    Ok(event)
}
