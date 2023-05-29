use crate::components::async_file_importer::FileFilter;
use crate::components::chat_contact::ChatInfo;
use crate::db::{
    DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, ProfileCache, UserConfig,
};
use crate::net::nostr_events::{add_relays_and_connect, create_channel, NostrState};
use crate::net::operations::builder::{build_contact_list_event, build_dm, build_profile_event};
use crate::net::operations::contact::{
    add_new_contact, delete_contact, fetch_and_decrypt_chat, update_contact,
};
use crate::net::reqwest_client::{download_image, fetch_latest_version};
use crate::net::ImageKind;
use crate::types::ChatMessage;
use crate::views::login::BasicProfile;
use crate::Error;
use futures::channel::mpsc;
use futures_util::SinkExt;
use nostr::secp256k1::XOnlyPublicKey;
use nostr::Keys;
use nostr_sdk::RelayOptions;
use rfd::AsyncFileDialog;
use sqlx::SqlitePool;
use std::path::PathBuf;

use super::{BackEndInput, BackendEvent, BackendState};

#[derive(Debug, Clone)]
pub enum ToBackend {
    Logout,
    // -------- REQWEST MESSAGES
    FetchLatestVersion,
    // -------- DATABASE MESSAGES
    QueryFirstLogin,
    PrepareClient,
    FetchRelayResponses(ChatMessage),
    FetchRelayResponsesUserProfile,
    FetchRelayResponsesContactList,
    FetchMessages(DbContact),

    FetchContacts,
    AddContact(DbContact),
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    ImportContacts((Vec<DbContact>, bool)),

    FetchRelays,
    CreateAccount(BasicProfile),
    GetUserProfileMeta,
    UpdateUserProfileMeta(nostr::Metadata),
    FetchAllMessageEvents,
    ExportMessages(Vec<DbEvent>),
    ExportContacts,
    FetchChatInfo(DbContact),
    GetDbEventWithHash(nostr::EventId),
    FetchSingleContact(XOnlyPublicKey),

    // -------- NOSTR CLIENT MESSAGES
    RefreshContactsProfile,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    SendDM((DbContact, String)),
    SendContactListToRelays,
    GetRelayStatusList,
    CreateChannel,
    DownloadImage {
        image_url: String,
        kind: ImageKind,
        public_key: XOnlyPublicKey,
    },
    FetchMoreMessages((DbContact, ChatMessage)),
    RemoveFileFromCache((ProfileCache, ImageKind)),
    ChooseFile(Option<FileFilter>),
}

pub async fn process_message(
    keys: &Keys,
    nostr: &mut NostrState,
    backend: &mut BackendState,
    message: ToBackend,
) -> Result<Option<BackendEvent>, Error> {
    if let (Some(db_client), Some(req_client)) = (&mut backend.db_client, &mut backend.req_client) {
        let pool = &db_client.pool;
        let cache_pool = &db_client.cache_pool;
        let event_opt = match message {
            // --- RFD ---
            ToBackend::ChooseFile(file_filter_opt) => {
                let mut rfd_instance = AsyncFileDialog::new().set_directory("/");

                if let Some(filter) = &file_filter_opt {
                    rfd_instance = rfd_instance.add_filter(
                        &filter.name,
                        &filter
                            .extensions
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<_>>(),
                    );
                }

                if let Some(file_handle) = rfd_instance.pick_file().await {
                    Some(BackendEvent::ChoosenFile(file_handle.path().to_owned()))
                } else {
                    None
                }
            }
            // ---- REQWEST ----
            ToBackend::FetchLatestVersion => {
                handle_fetch_last_version(req_client.clone(), backend.sender.clone())
            }
            ToBackend::DownloadImage {
                image_url,
                public_key,
                kind,
            } => spawn_download_image(backend.sender.clone(), public_key, kind, image_url),
            // ---- CONFIG ----
            ToBackend::Logout => {
                panic!("Logout should be processed outside here")
            }
            ToBackend::CreateAccount(profile) => {
                let profile_meta: nostr::Metadata = profile.into();
                let ns_event = build_profile_event(pool, keys, &profile_meta).await?;
                Some(
                    to_backend_channel(
                        &mut backend.sender,
                        BackEndInput::StorePendingMetadata((ns_event, profile_meta)),
                    )
                    .await?,
                )
            }
            ToBackend::UpdateUserProfileMeta(profile_meta) => {
                let ns_event = build_profile_event(pool, keys, &profile_meta).await?;
                Some(
                    to_backend_channel(
                        &mut backend.sender,
                        BackEndInput::StorePendingMetadata((ns_event, profile_meta)),
                    )
                    .await?,
                )
            }
            ToBackend::QueryFirstLogin => {
                if UserConfig::query_has_logged_in(pool).await? {
                    let relays = DbRelay::fetch(pool).await?;
                    let last_event =
                        DbEvent::fetch_last_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
                    UserConfig::store_first_login(pool).await?;
                    Some(add_relays_and_connect(&nostr.client, &keys, &relays, last_event).await?)
                } else {
                    Some(BackendEvent::FirstLogin)
                }
            }
            ToBackend::PrepareClient => {
                let relays = DbRelay::fetch(pool).await?;
                let last_event =
                    DbEvent::fetch_last_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
                UserConfig::store_first_login(pool).await?;
                Some(add_relays_and_connect(&nostr.client, &keys, &relays, last_event).await?)
            }
            // -------- DATABASE MESSAGES -------
            ToBackend::RemoveFileFromCache((cache, kind)) => {
                ProfileCache::remove_file(cache_pool, &cache, kind).await?;
                Some(BackendEvent::CacheFileRemoved((cache, kind)))
            }
            ToBackend::FetchMoreMessages((db_contact, first_message)) => {
                let msgs = DbMessage::fetch_more(pool, db_contact.pubkey(), first_message).await?;
                match msgs.is_empty() {
                    false => {
                        let mut chat_messages = vec![];
                        tracing::debug!("Decrypting messages");
                        for db_message in &msgs {
                            let chat_message =
                                ChatMessage::from_db_message(&keys, &db_message, &db_contact)?;
                            chat_messages.push(chat_message);
                        }

                        Some(BackendEvent::GotChatMessages((db_contact, chat_messages)))
                    }
                    // update nostr subscriber
                    true => None,
                }
            }
            ToBackend::FetchSingleContact(pubkey) => {
                let req = DbContact::fetch_one(pool, cache_pool, &pubkey).await?;
                Some(BackendEvent::GotSingleContact((pubkey, req)))
            }
            ToBackend::FetchRelayResponsesUserProfile => {
                if let Some(profile_event) =
                    DbEvent::fetch_last_kind_pubkey(pool, nostr::Kind::Metadata, &keys.public_key())
                        .await?
                {
                    let all_relays = DbRelay::fetch(pool).await?;
                    let responses =
                        DbRelayResponse::fetch_by_event(pool, profile_event.event_id()?).await?;
                    Some(BackendEvent::GotRelayResponsesUserProfile {
                        responses,
                        all_relays,
                    })
                } else {
                    None
                }
            }
            ToBackend::FetchRelayResponsesContactList => {
                if let Some(profile_event) = DbEvent::fetch_last_kind_pubkey(
                    pool,
                    nostr::Kind::ContactList,
                    &keys.public_key(),
                )
                .await?
                {
                    let all_relays = DbRelay::fetch(pool).await?;
                    let responses =
                        DbRelayResponse::fetch_by_event(pool, profile_event.event_id()?).await?;
                    Some(BackendEvent::GotRelayResponsesContactList {
                        responses,
                        all_relays,
                    })
                } else {
                    None
                }
            }
            ToBackend::FetchChatInfo(db_contact) => {
                let chat_info = if let Some(last_msg) =
                    DbMessage::fetch_chat_last(pool, &db_contact.pubkey()).await?
                {
                    let unseen_messages = 0;
                    let last_chat_msg = ChatMessage::from_db_message(keys, &last_msg, &db_contact)?;
                    Some(ChatInfo {
                        unseen_messages,
                        last_message: last_chat_msg.content,
                        last_message_time: last_msg.display_time(),
                    })
                } else {
                    None
                };

                Some(BackendEvent::GotChatInfo((db_contact, chat_info)))
            }
            ToBackend::ExportMessages(messages) => {
                let rfd_instance = AsyncFileDialog::new().set_directory("/");
                if let Some(file_handle) = rfd_instance.save_file().await {
                    messages_to_json_file(file_handle.path().to_path_buf(), &messages).await?;
                    Some(BackendEvent::ExportedMessagesSucessfully)
                } else {
                    Some(BackendEvent::ExportedMessagesToIdle)
                }
            }
            ToBackend::ExportContacts => {
                let rfd_instance = AsyncFileDialog::new().set_directory("/");
                if let Some(file_handle) = rfd_instance.save_file().await {
                    let mut path = file_handle.path().to_path_buf();
                    path.set_extension("json");
                    let list = DbContact::fetch(pool, cache_pool).await?;
                    let event = build_contact_list_event(pool, keys, &list).await?;
                    let json = event.as_json();
                    tokio::fs::write(path, json).await?;
                    Some(BackendEvent::ExportedContactsSucessfully)
                } else {
                    Some(BackendEvent::ExportedContactsToIdle)
                }
            }
            ToBackend::FetchAllMessageEvents => {
                let messages =
                    DbEvent::fetch_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
                Some(BackendEvent::GotAllMessages(messages))
            }
            ToBackend::GetUserProfileMeta => {
                tracing::info!("Fetching user profile meta");
                let cache =
                    ProfileCache::fetch_by_public_key(cache_pool, &keys.public_key()).await?;
                Some(BackendEvent::GotUserProfileCache(cache))
            }

            ToBackend::FetchRelayResponses(chat_message) => {
                if let Some(db_message) = DbMessage::fetch_one(pool, chat_message.msg_id).await? {
                    let all_relays = DbRelay::fetch(pool).await?;
                    let responses =
                        DbRelayResponse::fetch_by_event(pool, db_message.event_id()?).await?;
                    Some(BackendEvent::GotRelayResponses {
                        responses,
                        all_relays,
                        chat_message,
                    })
                } else {
                    None
                }
            }
            ToBackend::ImportContacts((db_contacts, is_replace)) => {
                for db_contact in &db_contacts {
                    if is_replace {
                        DbContact::delete(pool, db_contact).await?;
                        // todo: send event to front?
                        let _ = add_new_contact(keys, pool, db_contact).await;
                    } else {
                        let _ = update_contact(keys, pool, db_contact).await;
                    }
                }

                send_contact_list(pool, cache_pool, keys, &mut backend.sender).await?;

                Some(BackendEvent::FileContactsImported(db_contacts))
            }
            ToBackend::AddContact(db_contact) => {
                add_new_contact(keys, pool, &db_contact).await?;
                send_contact_list(pool, cache_pool, keys, &mut backend.sender).await?;
                Some(BackendEvent::ContactCreated(db_contact))
            }
            ToBackend::UpdateContact(db_contact) => {
                update_contact(&keys, pool, &db_contact).await?;
                send_contact_list(pool, cache_pool, keys, &mut backend.sender).await?;
                Some(BackendEvent::ContactUpdated(db_contact))
            }
            ToBackend::DeleteContact(db_contact) => {
                delete_contact(pool, &db_contact).await?;
                send_contact_list(pool, cache_pool, keys, &mut backend.sender).await?;
                Some(BackendEvent::ContactDeleted(db_contact))
            }
            ToBackend::FetchContacts => {
                let contacts = DbContact::fetch(pool, cache_pool).await?;
                Some(BackendEvent::GotContacts(contacts))
            }
            ToBackend::FetchRelays => {
                let relays = DbRelay::fetch(pool).await?;
                Some(BackendEvent::GotRelays(relays))
            }
            ToBackend::FetchMessages(contact) => {
                Some(fetch_and_decrypt_chat(&keys, pool, contact).await?)
            }
            ToBackend::GetDbEventWithHash(event_hash) => {
                let db_event_opt = DbEvent::fetch_one(pool, &event_hash).await?;
                Some(BackendEvent::GotDbEvent(db_event_opt))
            }
            // --------- NOSTR MESSAGES ------------
            ToBackend::GetRelayStatusList => {
                // let list = nostr.client.relay_status_list().await?;
                let mut list = vec![];
                for (url, relay) in nostr.client.relays().await {
                    list.push((url, relay.status().await))
                }
                Some(BackendEvent::GotRelayStatusList(list))
            }
            ToBackend::RefreshContactsProfile => {
                // subscribe_eose contact_list_metadata
                todo!();
            }
            ToBackend::CreateChannel => Some(create_channel(&nostr.client).await?),
            ToBackend::AddRelay(db_relay) => {
                tracing::debug!("Adding relay to client: {}", db_relay.url);
                let opts = RelayOptions::new(db_relay.read, db_relay.write);
                nostr
                    .client
                    .add_relay_with_opts(db_relay.url.as_str(), None, opts)
                    .await?;
                Some(
                    to_backend_channel(&mut backend.sender, BackEndInput::AddRelayToDb(db_relay))
                        .await?,
                )
            }
            ToBackend::DeleteRelay(db_relay) => {
                tracing::debug!("delete_relay");
                nostr.client.remove_relay(db_relay.url.as_str()).await?;
                Some(
                    to_backend_channel(
                        &mut backend.sender,
                        BackEndInput::DeleteRelayFromDb(db_relay),
                    )
                    .await?,
                )
            }
            ToBackend::ToggleRelayRead((db_relay, read)) => {
                // tracing::debug!("toggle_read_for_relay");
                // nostr.client.toggle_read_for(&db_relay.url, read)?;
                // Some(
                //     to_backend_channel(
                //         &mut backend.sender,
                //         BackEndInput::ToggleRelayRead((db_relay, read)),
                //     )
                //     .await?,
                // )
                None
            }
            ToBackend::ToggleRelayWrite((db_relay, write)) => {
                // tracing::debug!("toggle_write_for_relay");
                // nostr.client.toggle_write_for(&db_relay.url, write)?;
                // Some(
                //     to_backend_channel(
                //         &mut backend.sender,
                //         BackEndInput::ToggleRelayWrite((db_relay, write)),
                //     )
                //     .await?,
                // )
                None
            }
            // -- TO BACKEND CHANNEL --
            ToBackend::SendDM((db_contact, content)) => {
                // to backend channel, create a pending event and await confirmation of relays
                let ns_event = build_dm(pool, keys, &db_contact, &content).await?;
                Some(
                    to_backend_channel(
                        &mut backend.sender,
                        BackEndInput::StorePendingMessage {
                            ns_event,
                            content,
                            db_contact,
                        },
                    )
                    .await?,
                )
            }
            ToBackend::SendContactListToRelays => {
                let list = DbContact::fetch(pool, cache_pool).await?;
                let ns_event = build_contact_list_event(pool, &keys, &list).await?;
                Some(
                    to_backend_channel(
                        &mut backend.sender,
                        BackEndInput::StorePendingContactList((ns_event, list.to_owned())),
                    )
                    .await?,
                )
            }
        };
        Ok(event_opt)
    } else {
        Ok(None)
    }
}

async fn send_contact_list(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::UnboundedSender<BackEndInput>,
) -> Result<(), Error> {
    let list = DbContact::fetch(pool, cache_pool).await?;
    let ns_event = build_contact_list_event(pool, keys, &list).await?;
    to_backend_channel(
        back_sender,
        BackEndInput::StorePendingContactList((ns_event, list.to_owned())),
    )
    .await?;
    Ok(())
}

pub async fn to_backend_channel(
    ch: &mut mpsc::UnboundedSender<BackEndInput>,
    input: BackEndInput,
) -> Result<BackendEvent, Error> {
    ch.send(input)
        .await
        .map(|_| BackendEvent::BackendLoading)
        .map_err(|e| Error::FailedToSendBackendInput(e.to_string()))
}

fn spawn_download_image(
    mut back_sender: mpsc::UnboundedSender<BackEndInput>,
    public_key: XOnlyPublicKey,
    kind: ImageKind,
    image_url: String,
) -> Option<BackendEvent> {
    let event = BackendEvent::DownloadingImage {
        kind: kind.clone(),
        public_key: public_key.clone(),
    };

    tokio::spawn(async move {
        match download_image(&image_url, &public_key, kind).await {
            Ok(path) => {
                let msg = BackEndInput::ImageDownloaded {
                    kind: kind,
                    public_key: public_key,
                    path,
                };
                if let Err(e) = back_sender.send(msg).await {
                    tracing::error!("Error sending image downloaded event: {}", e);
                }
            }
            Err(e) => {
                let e = crate::Error::FromReqwestClientError(e);
                if let Err(e) = back_sender.send(BackEndInput::Error(e)).await {
                    tracing::error!("Error sending image download error event: {}", e);
                }
            }
        }
    });

    Some(event)
}

async fn messages_to_json_file(mut path: PathBuf, messages: &[DbEvent]) -> Result<(), Error> {
    path.set_extension("json");

    // Convert each DbEvent to a nostr::Event and collect into a Vec.
    let ns_events: Result<Vec<_>, _> = messages.iter().map(|m| m.to_ns_event()).collect();
    let ns_events = ns_events?; // Unwrap the Result, propagating any errors.

    // Convert the Vec<nostr::Event> into a JSON byte vector.
    let json = serde_json::to_vec(&ns_events)?;

    // Write the JSON byte vector to the file asynchronously.
    tokio::fs::write(path, json).await?;

    Ok(())
}

fn handle_fetch_last_version(
    req_client: reqwest::Client,
    mut sender: mpsc::UnboundedSender<BackEndInput>,
) -> Option<BackendEvent> {
    tracing::info!("Fetching latest version");
    tokio::spawn(async move {
        match fetch_latest_version(req_client).await {
            Ok(version) => {
                if let Err(e) = sender.send(BackEndInput::LatestVersion(version)).await {
                    tracing::error!("Error sending latest version to backend: {}", e);
                }
            }
            Err(e) => {
                if let Err(e) = sender
                    .send(BackEndInput::Error(Error::FailedToSendBackendInput(
                        e.to_string(),
                    )))
                    .await
                {
                    tracing::error!("Error sending error to backend: {}", e);
                }
            }
        }
    });
    Some(BackendEvent::FetchingLatestVersion)
}
