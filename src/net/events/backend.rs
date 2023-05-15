use std::path::PathBuf;

use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbRelay, UserConfig},
    error::Error,
    net::{
        events::nostr::NostrInput,
        operations::{
            download::download_profile_image,
            event::{
                insert_confirmed_event, insert_pending_contact_list, insert_pending_dm,
                insert_pending_metadata, on_relay_message,
            },
            metadata::{handle_profile_banner_update, handle_profile_picture_update},
        },
        process_async_with_event,
    },
};

use super::Event;

#[derive(Debug, Clone)]
pub enum BackEndInput {
    NtpTime(u64),
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
    ProfilePictureDownloaded((XOnlyPublicKey, PathBuf)),
    ProfileBannerDownloaded((XOnlyPublicKey, PathBuf)),
    Shutdown,
    FinishedPreparingNostr,
    Error(String),
    Ok(Event),
}

pub async fn backend_processing(
    pool: &SqlitePool,
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
        BackEndInput::FinishedPreparingNostr => {
            // donwload images?
            if let Ok(contact_list) = DbContact::fetch(pool).await {
                for c in contact_list {
                    if let Some(metadata) = c.get_profile_meta() {
                        download_profile_image(back_sender, &metadata, c.pubkey()).await
                    }
                }
            }
            Event::FinishedPreparing
        }
        BackEndInput::ProfilePictureDownloaded((public_key, path)) => {
            handle_profile_picture_update(keys, public_key, pool, path).await
        }
        BackEndInput::ProfileBannerDownloaded((public_key, path)) => {
            handle_profile_banner_update(keys, public_key, pool, path).await
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
                keys,
                back_sender,
                nostr_sender,
                &relay_url,
                ns_event,
            ))
            .await
        }
        BackEndInput::StoreRelayMessage((relay_url, relay_message)) => {
            process_async_with_event(on_relay_message(&pool, keys, &relay_url, &relay_message))
                .await
        }
    }
}

pub async fn handle_recommend_relay(ns_event: nostr_sdk::Event) -> Result<Event, Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&ns_event);
    Ok(Event::None)
}
