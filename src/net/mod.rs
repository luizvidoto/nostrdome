use crate::components::chat_contact::ChatInfo;
use crate::db::{
    Database, DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, ProfileCache, UserConfig,
};
use crate::error::Error;
use crate::net::operations::contact_list::handle_contact_list;
use crate::net::operations::direct_message::handle_dm;
use crate::net::operations::event::{confirm_event, confirmed_event, insert_other_kind};
use crate::net::operations::metadata::handle_metadata_event;
use crate::net::reqwest_client::fetch_latest_version;
use crate::ntp::spawn_ntp_request;
use crate::types::ChatMessage;
use crate::views::login::BasicProfile;
use futures::channel::mpsc;
use futures::StreamExt;
use futures_util::SinkExt;
use iced::subscription;
use iced::Subscription;
use nostr::secp256k1::XOnlyPublicKey;
use nostr::{Keys, RelayMessage};
use ns_client::NotificationEvent;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tokio::sync::broadcast;

mod back_channel;
mod backend_event;
mod nostr_events;
mod operations;
mod reqwest_client;
pub(crate) use backend_event::BackendEvent;

pub(crate) use reqwest_client::{image_filename, sized_image, ImageKind, ImageSize};

pub(crate) use self::back_channel::BackEndConnection;
use self::nostr_events::{prepare_client, NostrInput, NostrOutput, NostrSdkWrapper};
use self::operations::builder::{build_contact_list_event, build_dm, build_profile_event};
use self::operations::contact::{
    add_new_contact, delete_contact, fetch_and_decrypt_chat, update_contact,
};
use self::operations::event::{
    insert_pending_contact_list, insert_pending_dm, insert_pending_metadata,
};
use self::reqwest_client::download_image;

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

pub struct BackendState {
    db_client: Option<Database>,
    req_client: Option<reqwest::Client>,
    sender: mpsc::UnboundedSender<BackEndInput>,
    receiver: mpsc::UnboundedReceiver<BackEndInput>,
}
impl BackendState {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            db_client: None,
            req_client: None,
            sender,
            receiver,
        }
    }

    fn with_db(mut self, db_client: Database) -> Self {
        self.db_client = Some(db_client);
        self
    }
    fn with_req(mut self, req_client: reqwest::Client) -> Self {
        self.req_client = Some(req_client);
        self
    }

    pub async fn logout(self) -> Result<(), Error> {
        tracing::info!("Database Logging out");
        Ok(())
    }

    pub async fn process_notification(
        &mut self,
        notification: NotificationEvent,
    ) -> Option<BackendEvent> {
        // let backend_input = match notification {
        //     RelayPoolNotification::Event(relay_url, event) => {
        //         BackEndInput::StoreConfirmedEvent((relay_url, event))
        //     }
        //     RelayPoolNotification::Message(relay_url, msg) => {
        //         BackEndInput::StoreRelayMessage((relay_url, msg))
        //     }
        //     RelayPoolNotification::Shutdown => BackEndInput::Shutdown,
        // };

        let backend_input_opt = match notification {
            NotificationEvent::RelayTerminated(url) => {
                tracing::debug!("Relay terminated - {}", url);
                None
            }
            NotificationEvent::RelayMessage((relay_url, relay_msg)) => match relay_msg {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } => Some(BackEndInput::StoreConfirmedEvent((relay_url, *event))),
                other => Some(BackEndInput::StoreRelayMessage((relay_url, other))),
            },
            NotificationEvent::SentSubscription((url, sub_id)) => {
                tracing::debug!("Sent subscription to {} - id: {}", url, sub_id);
                None
            }
            NotificationEvent::SentEvent((url, event_hash)) => {
                tracing::debug!("Sent event to {} - hash: {}", url, event_hash);
                None
            }
        };

        if let Some(input) = backend_input_opt {
            if let Err(e) = self.sender.send(input).await {
                tracing::error!("{}", e);
            }
        }

        None
    }
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
                if let Err(e) = back_sender
                    .send(BackEndInput::Error(Error::FailedToSendBackendInput(
                        e.to_string(),
                    )))
                    .await
                {
                    tracing::error!("Error sending image download error event: {}", e);
                }
            }
        }
    });

    Some(event)
}

pub struct NostrState {
    keys: Keys,
    client: NostrSdkWrapper,
    notifications: broadcast::Receiver<NotificationEvent>,
}
impl NostrState {
    pub async fn new(keys: &Keys) -> Self {
        let client = NostrSdkWrapper::new(&keys).await;
        let notifications = client.notifications();
        Self {
            keys: keys.to_owned(),
            client,
            notifications,
        }
    }
    pub async fn logout(self) -> Result<(), Error> {
        Ok(self.client.logout().await?)
    }
    pub async fn process_in(&self, input: NostrInput) -> Result<Option<NostrOutput>, Error> {
        self.client.process_in(&self.keys, input).await
    }
}

pub enum State {
    End,
    Start {
        keys: Keys,
        backend: BackendState,
    },
    Connected {
        keys: Keys,
        receiver: mpsc::UnboundedReceiver<ToBackend>,
        backend: BackendState,
        nostr: NostrState,
    },
    OnlyNostr {
        keys: Keys,
        receiver: mpsc::UnboundedReceiver<ToBackend>,
        nostr: NostrState,
    },
}
impl State {
    pub fn start(keys: Keys) -> Self {
        let backend = BackendState::new();
        Self::Start { keys, backend }
    }
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<BackendEvent>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub = subscription::unfold(
        id,
        State::start(keys.to_owned()),
        |state| async move {
            match state {
                State::End => iced::futures::future::pending().await,
                State::OnlyNostr {
                    receiver: _,
                    nostr,
                    keys: _,
                } => {
                    if let Err(e) = nostr.logout().await {
                        tracing::error!("{}", e);
                    }
                    (BackendEvent::LoggedOut, State::End)
                }
                State::Start { keys, backend } => {
                    let (sender, receiver) = mpsc::unbounded();
                    let nostr = NostrState::new(&keys).await;
                    let req_client = reqwest::Client::new();
                    let db_client = match Database::new(&keys.public_key().to_string()).await {
                        Ok(database) => database,
                        Err(e) => {
                            tracing::error!("Failed to init database");
                            tracing::error!("{}", e);
                            tracing::warn!("Trying again in 2 secs");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            return (BackendEvent::Disconnected, State::Start { keys, backend });
                        }
                    };

                    let backend = backend.with_db(db_client).with_req(req_client);

                    tracing::info!("Fetching NTP time to sync with system time");
                    spawn_ntp_request(backend.sender.clone());

                    (
                        BackendEvent::Connected(BackEndConnection::new(sender)),
                        State::Connected {
                            receiver,
                            backend,
                            nostr,
                            keys,
                        },
                    )
                }
                State::Connected {
                    mut receiver,
                    mut backend,
                    mut nostr,
                    keys,
                } => {
                    let event_opt = tokio::select! {
                        message = receiver.select_next_some() => {
                            if let ToBackend::Logout = message {
                                if let Err(e) = backend.logout().await {
                                    tracing::error!("{}", e);
                                }
                                return (BackendEvent::BackendClosed, State::OnlyNostr { receiver, nostr, keys });
                            } else {
                                match process_message(&keys, &mut nostr, &mut backend, message).await {
                                    Ok(event_opt) => event_opt,
                                    Err(e) => Some(BackendEvent::Error(e.to_string()))
                                }
                            }
                        }
                        backend_input = backend.receiver.select_next_some() => {
                            match backend_processing(&keys, &mut nostr, &mut backend, backend_input).await {
                                Ok(event_opt) => event_opt,
                                Err(e) => Some(BackendEvent::Error(e.to_string()))
                            }
                        }
                        notification = nostr.notifications.recv() => {
                            if let Ok(notification) = notification{
                                backend.process_notification(notification).await
                            } else {
                                None
                            }
                        },
                    };
                    let event = match event_opt {
                        Some(event) => event,
                        None => BackendEvent::Empty,
                    };
                    (
                        event,
                        State::Connected {
                            receiver,
                            backend,
                            nostr,
                            keys,
                        },
                    )
                }
            }
        },
    );

    vec![database_sub]
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

// ---------------------------------
// ---------------------------------
// ---------------------------------

pub async fn to_backend_channel(
    ch: &mut mpsc::UnboundedSender<BackEndInput>,
    input: BackEndInput,
) -> Result<BackendEvent, Error> {
    ch.send(input)
        .await
        .map(|_| BackendEvent::BackendLoading)
        .map_err(|e| Error::FailedToSendBackendInput(e.to_string()))
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
    StoreConfirmedEvent((nostr::Url, nostr::Event)),
    StoreRelayMessage((nostr::Url, nostr::RelayMessage)),
    LatestVersion(String),
    ImageDownloaded {
        kind: ImageKind,
        public_key: XOnlyPublicKey,
        path: PathBuf,
    },
    FinishedPreparingNostr,
    Error(Error),
    Ok(Option<BackendEvent>),
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
            BackEndInput::Ok(event) => event,
            BackEndInput::Error(e) => return Err(e),
            // --- REQWEST ---
            BackEndInput::LatestVersion(version) => Some(BackendEvent::LatestVersion(version)),
            // --- TO DATABASE ---
            BackEndInput::NtpTime(total_microseconds) => {
                tracing::info!("NTP time: {}", total_microseconds);
                UserConfig::update_ntp_offset(pool, total_microseconds).await?;
                Some(BackendEvent::SyncedWithNtpServer)
            }
            BackEndInput::FinishedPreparingNostr => Some(BackendEvent::FinishedPreparing),
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
            BackEndInput::StoreConfirmedEvent((relay_url, ns_event)) => {
                let result = match ns_event.kind {
                    nostr::Kind::ContactList => {
                        // contact_list is validating differently
                        handle_contact_list(
                            ns_event,
                            keys,
                            pool,
                            backend.sender.clone(),
                            nostr,
                            &relay_url,
                        )
                        .await
                    }
                    other => {
                        if let Some(db_event) = confirmed_event(ns_event, &relay_url, pool).await? {
                            match other {
                                nostr::Kind::Metadata => {
                                    handle_metadata_event(cache_pool, db_event).await
                                }
                                nostr::Kind::EncryptedDirectMessage => {
                                    handle_dm(pool, cache_pool, keys, &relay_url, db_event).await
                                }
                                nostr::Kind::RecommendRelay => {
                                    handle_recommend_relay(db_event).await
                                }
                                // nostr::Kind::ChannelCreation => {}
                                // nostr::Kind::ChannelMetadata => {
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
                    tracing::debug!("Relay message: EOSE. ID: {}", subscription_id);
                    Some(BackendEvent::EndOfStoredEvents((
                        relay_url.to_owned(),
                        subscription_id.to_owned(),
                    )))
                }
                RelayMessage::Event {
                    subscription_id,
                    event,
                } => {
                    tracing::debug!("Relay message: Event. ID: {}", subscription_id);
                    tracing::debug!("{:?}", event);
                    None
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
            handle_dm_confirmation(pool, cache_pool, keys, db_event).await
        }
        _ => Err(Error::NotSubscribedToKind(db_event.kind.clone())),
    }
}

async fn handle_dm_confirmation(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<BackendEvent, Error> {
    let relay_url = db_event
        .relay_url
        .as_ref()
        .ok_or(Error::NotConfirmedEvent(db_event.event_hash.to_owned()))?;

    if let Some(db_message) = DbMessage::fetch_by_event(pool, db_event.event_id()?).await? {
        let db_message = DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
        if let Some(db_contact) =
            DbContact::fetch_one(pool, cache_pool, &db_message.contact_chat()).await?
        {
            let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
            return Ok(BackendEvent::ConfirmedDM((db_contact, chat_message)));
        } else {
            tracing::error!("No contact found for confirmed message");
            return Err(Error::ContactNotFound(db_message.contact_chat().to_owned()));
        }
    }

    tracing::error!("No message found for confirmation event");
    Err(Error::EventWithoutMessage(db_event.event_id()?))
}

pub async fn update_profile_cache(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    public_key: XOnlyPublicKey,
    kind: ImageKind,
    path: PathBuf,
) -> Result<BackendEvent, Error> {
    let _ = ProfileCache::update_local_path(cache_pool, &public_key, kind, &path).await?;

    let event = match keys.public_key() == public_key {
        true => match kind {
            ImageKind::Profile => BackendEvent::UserProfilePictureUpdated(path),
            ImageKind::Banner => BackendEvent::UserBannerPictureUpdated(path),
        },
        false => {
            if let Some(db_contact) = DbContact::fetch_one(pool, cache_pool, &public_key).await? {
                BackendEvent::ContactUpdated(db_contact)
            } else {
                tracing::warn!(
                    "Image downloaded for contact not found in database: {}",
                    public_key
                );
                return Err(Error::ContactNotFound(public_key.clone()));
            }
        }
    };

    Ok(event)
}
