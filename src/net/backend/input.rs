use crate::db::channel_cache::ChannelCache;
use crate::db::profile_cache::ProfileCache;
use crate::db::{DbContact, DbEvent, DbRelay, UserConfig};
use crate::net::nostr_events::{contact_list_metadata, NostrState, SubscriptionType};
use crate::net::operations::cache::update_profile_cache;
use crate::net::operations::contact_list::handle_contact_list;
use crate::net::operations::direct_message::{handle_dm, handle_dm_confirmation};
use crate::net::operations::event::{
    confirm_event, confirmed_event, insert_other_kind, insert_pending_contact_list,
    insert_pending_dm, insert_pending_metadata,
};
use crate::net::ImageKind;
use crate::Error;
use sqlx::SqlitePool;

use nostr::secp256k1::XOnlyPublicKey;
use nostr::{Keys, RelayMessage, SubscriptionId};
use std::path::PathBuf;

use super::{BackendEvent, BackendState};

#[derive(Debug)]
pub enum BackEndInput {
    NtpTime(u64),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    StorePendingContactList((nostr::Event, Vec<DbContact>)),
    StorePendingMetadata((nostr::Event, nostr::Metadata)),
    StorePendingMessage {
        ns_event: nostr::Event,
        db_contact: DbContact,
        content: String,
    },
    StoreRelayMessage((nostr::Url, nostr::RelayMessage)),
    LatestVersion(String),
    ImageDownloaded {
        kind: ImageKind,
        public_key: XOnlyPublicKey,
        path: PathBuf,
    },
    Error(Error),
    Ok(BackendEvent),
}

pub async fn backend_processing(
    keys: &Keys,
    nostr: &mut NostrState,
    backend: &mut BackendState,
    input: BackEndInput,
) -> Result<Option<BackendEvent>, Error> {
    if let (Some(db_client), Some(_req_client)) = (&mut backend.db_client, &mut backend.req_client)
    {
        let pool = &db_client.pool;
        let cache_pool = &db_client.cache_pool;
        let event_opt = match input {
            BackEndInput::Ok(event) => Some(event),
            BackEndInput::Error(e) => return Err(e),
            // --- REQWEST ---
            BackEndInput::LatestVersion(version) => Some(BackendEvent::LatestVersion(version)),
            // --- TO DATABASE ---
            BackEndInput::NtpTime(total_microseconds) => {
                tracing::info!("NTP time: {}", total_microseconds);
                UserConfig::update_ntp_offset(pool, total_microseconds).await?;
                Some(BackendEvent::SyncedWithNtpServer)
            }
            BackEndInput::ImageDownloaded {
                kind,
                public_key,
                path,
            } => Some(update_profile_cache(pool, cache_pool, keys, public_key, kind, path).await?),
            BackEndInput::DeleteRelayFromDb(db_relay) => {
                DbRelay::delete(pool, &db_relay).await?;
                Some(BackendEvent::RelayDeleted(db_relay))
            }
            BackEndInput::StorePendingMessage {
                ns_event,
                db_contact,
                content,
            } => Some(insert_pending_dm(pool, keys, ns_event, db_contact, content, nostr).await?),
            BackEndInput::StorePendingContactList((ns_event, contact_list)) => {
                Some(insert_pending_contact_list(pool, ns_event, contact_list, nostr).await?)
            }
            BackEndInput::StorePendingMetadata((ns_event, metadata)) => {
                Some(insert_pending_metadata(pool, ns_event, metadata, nostr).await?)
            }
            BackEndInput::ToggleRelayRead((mut db_relay, read)) => {
                db_relay.read = read;
                DbRelay::update(&pool, &db_relay).await?;
                Some(BackendEvent::RelayUpdated(db_relay.clone()))
            }
            BackEndInput::ToggleRelayWrite((mut db_relay, write)) => {
                db_relay.write = write;
                DbRelay::update(&pool, &db_relay).await?;
                Some(BackendEvent::RelayUpdated(db_relay.clone()))
            }
            BackEndInput::AddRelayToDb(db_relay) => {
                DbRelay::insert(pool, &db_relay).await?;
                Some(BackendEvent::RelayCreated(db_relay))
            }
            BackEndInput::StoreRelayMessage((relay_url, relay_message)) => match relay_message {
                RelayMessage::Ok {
                    event_id: event_hash,
                    status,
                    message,
                } => {
                    if status == false {
                        let db_relay =
                            DbRelay::update_with_error(pool, &relay_url, &message).await?;
                        return Ok(Some(BackendEvent::RelayUpdated(db_relay)));
                    }
                    tracing::debug!("Relay message: Ok");
                    let db_event = confirm_event(pool, &event_hash, &relay_url).await?;
                    Some(on_event_confirmation(pool, cache_pool, keys, &db_event).await?)
                }
                RelayMessage::EndOfStoredEvents(subscription_id) => {
                    if let Some(sub_type) = backend.subscriptions.get(&subscription_id) {
                        match sub_type {
                            SubscriptionType::ContactList => {
                                let list = DbContact::fetch(pool, cache_pool).await?;
                                let id = SubscriptionId::new(
                                    SubscriptionType::ContactListMetadata.to_string(),
                                );
                                let filters = vec![contact_list_metadata(&list)];
                                if let Ok(relay) = nostr.client.relay(&relay_url).await {
                                    relay.req_events_of(filters, None);
                                }
                            }
                            // SubscriptionType::Messages => todo!(),
                            // SubscriptionType::ContactListMetadata => todo!(),
                            // SubscriptionType::Channel => todo!(),
                            // SubscriptionType::ChannelMetadata => todo!(),
                            _other => (),
                        }
                    }
                    None
                }
                RelayMessage::Event {
                    subscription_id: _,
                    event: ns_event,
                } => {
                    let result = match ns_event.kind {
                        nostr::Kind::ContactList => {
                            // contact_list is validating differently
                            handle_contact_list(
                                *ns_event,
                                keys,
                                pool,
                                backend.sender.clone(),
                                &relay_url,
                            )
                            .await
                        }
                        other => {
                            if let Some(db_event) =
                                confirmed_event(*ns_event, &relay_url, pool).await?
                            {
                                match other {
                                    nostr::Kind::Metadata => {
                                        handle_metadata_event(cache_pool, &db_event).await
                                    }
                                    nostr::Kind::EncryptedDirectMessage => {
                                        handle_dm(pool, cache_pool, keys, &relay_url, &db_event)
                                            .await
                                    }
                                    nostr::Kind::RecommendRelay => {
                                        handle_recommend_relay(db_event).await
                                    }
                                    nostr::Kind::ChannelCreation => {
                                        handle_channel_creation(cache_pool, &db_event).await
                                    }
                                    nostr::Kind::ChannelMetadata => {
                                        handle_channel_update(cache_pool, &db_event).await
                                    }
                                    // nostr::Kind::ChannelMessage => {}
                                    // nostr::Kind::ChannelHideMessage => {}
                                    // nostr::Kind::ChannelMuteUser => {}
                                    // Kind::EventDeletion => {},
                                    // Kind::PublicChatReserved45 => {},
                                    // Kind::PublicChatReserved46 => {},
                                    // Kind::PublicChatReserved47 => {},
                                    // Kind::PublicChatReserved48 => {},
                                    // Kind::PublicChatReserved49 => {},
                                    // Kind::ZapRequest => {},
                                    // Kind::Zap => {},
                                    // Kind::MuteList => {},
                                    // Kind::PinList => {},
                                    // Kind::RelayList => {},
                                    // Kind::Authentication => {},
                                    _other_kind => insert_other_kind(db_event).await,
                                }
                            } else {
                                Ok(None)
                            }
                        }
                    };
                    return result;
                }
                RelayMessage::Notice { message } => {
                    tracing::info!("Relay message: Notice: {}", message);
                    None
                }
                RelayMessage::Auth { challenge } => {
                    tracing::warn!("Relay message: Auth Challenge: {}", challenge);
                    None
                }
                RelayMessage::Count {
                    subscription_id: _,
                    count,
                } => {
                    tracing::info!("Relay message: Count: {}", count);
                    None
                }
                RelayMessage::Empty => {
                    tracing::info!("Relay message: Empty");
                    None
                }
            },
        };
        Ok(event_opt)
    } else {
        Ok(None)
    }
}

async fn handle_metadata_event(
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<Option<BackendEvent>, Error> {
    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::trace!("{:?}", db_event);

    let rows_changed = ProfileCache::insert_with_event(cache_pool, db_event).await?;

    if rows_changed == 0 {
        tracing::debug!("Cache already up to date");
    }

    Ok(Some(BackendEvent::UpdatedMetadata(db_event.pubkey)))
}

async fn handle_channel_creation(
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("handle_channel_creation");
    let channel_cache = ChannelCache::insert(cache_pool, db_event).await?;
    Ok(Some(BackendEvent::ChannelCreated(channel_cache)))
}

async fn handle_channel_update(
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("handle_channel_update");
    ChannelCache::update(cache_pool, db_event).await?;
    let channel_cache = ChannelCache::update(cache_pool, db_event).await?;
    Ok(Some(BackendEvent::ChannelUpdated(channel_cache)))
}

pub async fn handle_recommend_relay(db_event: DbEvent) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&db_event);
    Ok(None)
}

pub async fn on_event_confirmation(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<BackendEvent, Error> {
    match db_event.kind {
        nostr::Kind::ContactList => Ok(BackendEvent::ConfirmedContactList(db_event.to_owned())),
        nostr::Kind::Metadata => {
            let is_user = db_event.pubkey == keys.public_key();
            Ok(BackendEvent::ConfirmedMetadata {
                db_event: db_event.to_owned(),
                is_user,
            })
        }
        nostr::Kind::EncryptedDirectMessage => {
            Ok(handle_dm_confirmation(pool, cache_pool, keys, db_event).await?)
        }
        _ => Err(Error::NotSubscribedToKind(db_event.kind.clone())),
    }
}
