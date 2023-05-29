use crate::components::chat_contact::ChatInfo;
use crate::db::{
    DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, ProfileCache, UserConfig,
};
use crate::net::nostr_events::{prepare_client, NostrInput, NostrOutput, NostrState};
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
    ExportMessages((Vec<DbEvent>, PathBuf)),
    ExportContacts(std::path::PathBuf),
    FetchChatInfo(DbContact),
    GetDbEventWithHash(nostr::EventId),
    FetchSingleContact(XOnlyPublicKey),

    // -------- NOSTR CLIENT MESSAGES
    RequestEventsOf(DbRelay),
    RefreshContactsProfile,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    SendDM((DbContact, String)),
    SendContactListToRelays,
    GetRelayStatusList,
    GetRelayStatus(url::Url),
    CreateChannel,
    DownloadImage {
        image_url: String,
        kind: ImageKind,
        public_key: XOnlyPublicKey,
    },
    FetchMoreMessages((DbContact, ChatMessage)),
    RemoveFileFromCache((ProfileCache, ImageKind)),
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
                    let input = prepare_client(pool, cache_pool).await?;
                    let output = nostr.process_in(input).await?;
                    process_nostr_output(output, backend).await
                } else {
                    Some(BackendEvent::FirstLogin)
                }
            }
            ToBackend::PrepareClient => {
                let input = prepare_client(pool, cache_pool).await?;
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
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
            ToBackend::ExportMessages((messages, path)) => {
                messages_to_json_file(path, &messages).await?;
                Some(BackendEvent::ExportedMessagesSucessfully)
            }
            ToBackend::ExportContacts(mut path) => {
                path.set_extension("json");
                let list = DbContact::fetch(pool, cache_pool).await?;
                let event = build_contact_list_event(pool, keys, &list).await?;
                let json = event.as_json();
                tokio::fs::write(path, json).await?;
                Some(BackendEvent::ExportedContactsSucessfully)
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
                Some(BackendEvent::FileContactsImported(db_contacts))
            }
            ToBackend::AddContact(db_contact) => add_new_contact(keys, pool, &db_contact).await?,
            ToBackend::UpdateContact(db_contact) => {
                update_contact(&keys, pool, &db_contact).await?
            }
            ToBackend::DeleteContact(contact) => delete_contact(pool, &contact).await?,
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
            ToBackend::GetRelayStatus(url) => {
                todo!()
            }
            ToBackend::GetRelayStatusList => {
                let input = NostrInput::GetRelayStatusList;
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::RequestEventsOf(db_relay) => {
                let contact_list = DbContact::fetch(pool, cache_pool).await?;
                let input = NostrInput::RequestEventsOf((db_relay, contact_list));
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }

            ToBackend::RefreshContactsProfile => {
                let db_contacts = DbContact::fetch(pool, cache_pool).await?;
                let input = NostrInput::GetContactListProfiles(db_contacts);
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::CreateChannel => {
                let input = NostrInput::CreateChannel;
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::AddRelay(db_relay) => {
                let input = NostrInput::AddRelay(db_relay);
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::DeleteRelay(db_relay) => {
                let input = NostrInput::DeleteRelay(db_relay);
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::ToggleRelayRead((db_relay, read)) => {
                let input = NostrInput::ToggleRelayRead((db_relay, read));
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
            }
            ToBackend::ToggleRelayWrite((db_relay, write)) => {
                let input = NostrInput::ToggleRelayWrite((db_relay, write));
                let output = nostr.process_in(input).await?;
                process_nostr_output(output, backend).await
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

async fn process_nostr_output(
    output: Option<NostrOutput>,
    backend: &mut BackendState,
) -> Option<BackendEvent> {
    if let Some(o) = output {
        match o {
            NostrOutput::ToFrontEnd(event) => return Some(event),
            NostrOutput::ToBackend(back_event) => {
                if let Err(e) = backend.sender.send(back_event).await {
                    tracing::debug!("Failed to send event to backend channel: {}", e);
                }
            }
        }
    }
    None
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
