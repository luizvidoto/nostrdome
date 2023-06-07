use futures_util::SinkExt;
use iced::subscription;
use iced::Subscription;
use nostr::Metadata;
use rfd::AsyncFileDialog;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::broadcast;
use url::Url;

use nostr::secp256k1::XOnlyPublicKey;
use nostr::EventId;
use nostr::Keys;
use nostr::Kind;
use nostr::RelayMessage;
use nostr::SubscriptionId;

use ns_client::NotificationEvent;
use ns_client::RelayEvent;
use ns_client::RelayPool;

use crate::components::async_file_importer::FileFilter;
use crate::components::chat_contact::ChatInfo;
use crate::consts::NIPS_LIST_MARKDOWN;
use crate::db::ChannelCache;
use crate::db::Database;
use crate::db::DbContact;
use crate::db::DbEvent;
use crate::db::DbMessage;
use crate::db::DbRelay;
use crate::db::DbRelayResponse;
use crate::db::ImageDownloaded;
use crate::db::ProfileCache;
use crate::db::TagInfo;
use crate::db::UserConfig;
use crate::net::filters::channel_search_filter;
use crate::net::filters::contact_list_filter;
use crate::net::filters::messages_filter;
use crate::net::filters::user_metadata;
use crate::net::ntp::spawn_ntp_request;
use crate::net::operations::contact_list::received_contact_list;
use crate::net::reqwest_client::fetch_latest_version;
use crate::types::BackendState;
use crate::types::ChannelResult;
use crate::types::ChatMessage;
use crate::types::PrefixedId;
use crate::types::SubscriptionType;
use crate::utils::parse_nips_markdown;
use crate::utils::NipData;
use crate::views::login::BasicProfile;
use crate::Error;
mod filters;
pub(crate) mod ntp;
pub(crate) mod operations;
pub(crate) mod reqwest_client;
use self::filters::channel_details_filter;
use self::filters::contact_list_metadata;
use self::operations::contact_list::handle_contact_list;
use self::operations::direct_message::handle_dm;
use self::reqwest_client::download_image;
pub(crate) use reqwest_client::{image_filename, ImageKind, ImageSize};

#[derive(Debug, Clone)]
pub struct BackEndConnection(tokio::sync::mpsc::Sender<ToBackend>);
impl BackEndConnection {
    pub fn new(sender: tokio::sync::mpsc::Sender<ToBackend>) -> Self {
        Self(sender)
    }
    pub fn send(&mut self, input: ToBackend) {
        if let Err(e) = self.0.try_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}

pub enum State {
    Start,
    Ready(tokio::sync::mpsc::Receiver<ToBackend>),
}
pub enum ClientState {
    Empty,
    Connected {
        tasks_rx: tokio::sync::mpsc::Receiver<Result<TaskOutput, Error>>,
        tasks_tx: tokio::sync::mpsc::Sender<Result<TaskOutput, Error>>,
        keys: Keys,
        backend: BackendState,
        notifications: broadcast::Receiver<NotificationEvent>,
    },
}

pub enum TaskOutput {
    Ntp(u64),
    LatestVersion(String),
    ImageDownloaded(ImageDownloaded),
}

async fn handle_eose(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    _keys: &Keys,
    backend: &mut BackendState,
    url: Url,
    subscription_id: SubscriptionId,
) -> Result<(), Error> {
    // tracing::info!("EOSE {} - {}", &url, &subscription_id);

    if let Some(sub_type) = SubscriptionType::from_id(&subscription_id) {
        let pool = &backend.db_client.pool;
        let cache_pool = &backend.db_client.cache_pool;

        match sub_type {
            SubscriptionType::ContactList => {
                let list = DbContact::fetch(pool, cache_pool).await?;
                if let Some(filter) = contact_list_metadata(&list) {
                    backend.nostr.relay_subscribe_eose(
                        &url,
                        &SubscriptionType::ContactListMetadata.id(),
                        vec![filter],
                        Some(Duration::from_secs(10)),
                    )?;
                }
            }
            SubscriptionType::SearchChannels => {
                // send search channels metadata now
                let actions_id = SubscriptionId::generate().to_string();
                let actions: Vec<_> = backend
                    .search_channel_ids
                    .iter()
                    .map(|channel_id| {
                        ns_client::Subscription::action(channel_details_filter(channel_id))
                            .with_id(&SubscriptionType::src_channel_details(channel_id).id())
                            .timeout(Some(Duration::from_secs(5)))
                    })
                    .collect();
                backend
                    .nostr
                    .relay_eose_actions(&url, &actions_id, actions)?;
                _ = output
                    .send(BackendEvent::EOSESearchChannels(url.to_owned()))
                    .await;
            }
            SubscriptionType::SearchChannelsDetails(channel_id) => {
                _ = output
                    .send(BackendEvent::EOSESearchChannelsDetails(
                        url.to_owned(),
                        channel_id,
                    ))
                    .await;
            }
            _other => (),
        }
    }
    Ok(())
}

async fn handle_event(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    backend: &mut BackendState,
    url: Url,
    subscription_id: SubscriptionId,
    ns_event: nostr::Event,
) -> Result<(), Error> {
    tracing::debug!("Event {} - {} - {:?}", &url, &subscription_id, &ns_event);

    let pool = &backend.db_client.pool;
    let cache_pool = &backend.db_client.cache_pool;

    if let Some(sub_type) = SubscriptionType::from_id(&subscription_id) {
        match sub_type {
            SubscriptionType::SearchChannels => {
                backend.search_channel_ids.insert(ns_event.id.to_owned());
                let cache = ChannelCache::fetch_insert(cache_pool, &ns_event).await?;
                let result = ChannelResult::from_ns_event(url.to_owned(), ns_event, cache);
                _ = output.send(BackendEvent::SearchChannelResult(result)).await;
                return Ok(());
            }
            SubscriptionType::SearchChannelsDetails(channel_id) => {
                if let Kind::ChannelMetadata = ns_event.kind {
                    let cache = ChannelCache::update(cache_pool, &ns_event).await?;
                    _ = output
                        .send(BackendEvent::SearchChannelMetaUpdate(channel_id, cache))
                        .await;
                } else if let Kind::ChannelMessage = ns_event.kind {
                    _ = output
                        .send(BackendEvent::SearchChannelMessage(
                            channel_id,
                            ns_event.pubkey,
                        ))
                        .await;
                }
                return Ok(());
            }
            _ => (),
        }
    }

    match ns_event.kind {
        Kind::ContactList => {
            if let Some(db_event) = received_contact_list(pool, &url, &ns_event).await? {
                handle_contact_list(output, keys, pool, &url, db_event).await?;
            }
        }
        other => {
            if let Some(db_event) = DbEvent::insert(pool, &url, &ns_event).await? {
                match other {
                    Kind::Metadata => {
                        handle_metadata_event(output, cache_pool, &db_event).await?;
                    }
                    Kind::EncryptedDirectMessage => {
                        handle_dm(output, pool, cache_pool, keys, &url, &db_event).await?;
                    }
                    Kind::RecommendRelay => {
                        handle_recommend_relay(db_event).await?;
                    }
                    Kind::ChannelCreation => {
                        handle_channel_creation(output, cache_pool, &db_event).await?;
                    }
                    Kind::ChannelMetadata => {
                        handle_channel_update(output, cache_pool, &db_event).await?;
                    }
                    Kind::ChannelMessage => {
                        handle_channel_message(output, cache_pool, &db_event).await?;
                    }
                    _other_kind => {
                        _ = output
                            .send(BackendEvent::OtherKindEventInserted(db_event))
                            .await;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_relay_message(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    backend: &mut BackendState,
    _task_tx: &tokio::sync::mpsc::Sender<Result<TaskOutput, Error>>,
    url: Url,
    message: RelayMessage,
) -> Result<(), Error> {
    let pool = &backend.db_client.pool;
    let cache_pool = &backend.db_client.cache_pool;
    match message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message: error_msg,
        } => {
            // Status false means that some event was not accepted by the relay for some reason
            tracing::debug!(
                "{} - Ok: ID: {} --- {} - {}",
                &url,
                event_hash,
                status,
                &error_msg
            );

            if status == false {
                _ = output.send(BackendEvent::RelayError(url, error_msg)).await;
                return Ok(());
            }

            if let Some(pending) = backend.pending_events.remove(&event_hash) {
                if let Some(db_event) = DbEvent::insert(pool, &url, pending.ns_event()).await? {
                    match db_event.kind {
                        Kind::ContactList => {
                            _ = output
                                .send(BackendEvent::ConfirmedContactList(db_event.to_owned()))
                                .await;
                        }
                        Kind::Metadata => {
                            let is_user = db_event.pubkey == keys.public_key();
                            _ = output
                                .send(BackendEvent::ConfirmedMetadata {
                                    db_event: db_event.to_owned(),
                                    is_user,
                                })
                                .await;
                        }
                        Kind::EncryptedDirectMessage => {
                            let db_message = DbMessage::confirm_message(pool, &db_event).await?;
                            let db_contact = DbContact::fetch_insert(
                                pool,
                                cache_pool,
                                &db_message.contact_pubkey,
                            )
                            .await?;
                            let tag_info = TagInfo::from_db_event(&db_event)?;
                            let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
                            let chat_message =
                                ChatMessage::new(&db_message, &db_contact, &decrypted_content)?;
                            _ = output
                                .send(BackendEvent::ConfirmedDM((db_contact, chat_message)))
                                .await;
                            return Ok(());
                        }
                        _ => {
                            return Err(Error::NotSubscribedToKind(db_event.kind.clone()));
                        }
                    }
                }
            }
        }
        RelayMessage::EndOfStoredEvents(subscription_id) => {
            handle_eose(output, keys, backend, url, subscription_id).await?;
        }
        RelayMessage::Event {
            subscription_id,
            event: ns_event,
        } => {
            handle_event(output, keys, backend, url, subscription_id, *ns_event).await?;
        }
        RelayMessage::Notice { message } => {
            tracing::info!("Relay message: Notice: {}", message);
        }
        RelayMessage::Auth { challenge } => {
            tracing::warn!("Relay message: Auth Challenge: {}", challenge);
        }
        RelayMessage::Count {
            subscription_id: _,
            count,
        } => {
            tracing::info!("Relay message: Count: {}", count);
        }
        RelayMessage::Empty => {
            tracing::info!("Relay message: Empty");
        }
    }

    Ok(())
}

async fn get_clients(
    keys: &Keys,
    create_account: Option<BasicProfile>,
) -> Result<ClientState, Error> {
    let db_client = Database::new(&keys.public_key().to_string()).await?;
    let (tasks_tx, tasks_rx) = tokio::sync::mpsc::channel(100);
    let req_client = reqwest::Client::new();
    let nostr = RelayPool::new();
    let notifications = nostr.notifications();
    let nips_data = parse_nips_markdown(NIPS_LIST_MARKDOWN)?;
    let backend = BackendState::new(db_client, req_client, nostr, nips_data, create_account);

    spawn_ntp_request(tasks_tx.clone());

    Ok(ClientState::Connected {
        tasks_rx,
        tasks_tx,
        keys: keys.to_owned(),
        backend,
        notifications,
    })
}

pub fn backend_connect() -> Subscription<BackendEvent> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    subscription::channel(id, BACKEND_CHANNEL_SIZE, |mut output| async move {
        let mut state = State::Start;
        let mut client_state = ClientState::Empty;
        loop {
            match &mut state {
                State::Start => {
                    let (sender, receiver) =
                        tokio::sync::mpsc::channel::<ToBackend>(BACKEND_CHANNEL_SIZE);
                    state = State::Ready(receiver);
                    _ = output
                        .send(BackendEvent::Connected(BackEndConnection::new(sender)))
                        .await;
                }
                State::Ready(receiver) => {
                    match &mut client_state {
                        ClientState::Empty => {
                            if let Some(input) = receiver.recv().await {
                                match input {
                                    ToBackend::LoginWithSK(keys) => {
                                        match get_clients(&keys, None).await {
                                            Ok(state) => {
                                                client_state = state;
                                                let _ =
                                                    output.send(BackendEvent::LoginSuccess).await;
                                            }
                                            Err(e) => {
                                                tracing::error!("{}", e);
                                                _ = output
                                                    .send(BackendEvent::FailedToStartClient)
                                                    .await;
                                            }
                                        }
                                    }
                                    // TODO: do something with profile
                                    // create profile to send later to a relay when connected
                                    ToBackend::CreateAccount(profile) => {
                                        let keys = Keys::generate();
                                        match get_clients(&keys, Some(profile)).await {
                                            Ok(state) => {
                                                client_state = state;
                                                _ = output
                                                    .send(BackendEvent::CreateAccountSuccess)
                                                    .await;
                                            }
                                            Err(e) => {
                                                tracing::error!("{}", e);
                                                _ = output
                                                    .send(BackendEvent::FailedToStartClient)
                                                    .await;
                                            }
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        }
                        ClientState::Connected {
                            tasks_rx,
                            tasks_tx,
                            backend,
                            keys,
                            notifications,
                        } => {
                            tokio::select! {
                                message = receiver.recv() => {
                                    tracing::debug!("Received message from frontend");
                                    if let Some(message) = message {
                                        if let ToBackend::Logout = message {
                                            let _ = backend.logout().await;
                                            _ = output.send(BackendEvent::LogoutSuccess).await;
                                            state = State::Start;
                                            client_state = ClientState::Empty;

                                        } else {
                                            if let Err(e) = process_message(&mut output, &keys, backend, tasks_tx, message).await {
                                                // depending on the error, restart backend?
                                                tracing::error!("{}", e);
                                            }
                                        }
                                    } else {
                                        tracing::info!("Front to backend channel closed");
                                        let _ = backend.logout().await;
                                        _ = output.send(BackendEvent::LogoutSuccess).await;
                                        state = State::Start;
                                        client_state = ClientState::Empty;
                                    }
                                }
                                notification = notifications.recv() => {
                                    tracing::debug!("Received notification from nostr");
                                    if let Ok(notification) = notification {
                                        let url = notification.url;
                                        match notification.event {
                                            RelayEvent::ActionsDone(actions_id) => {
                                                tracing::info!("Actions done - {} - {}", &url, &actions_id);
                                            }
                                            RelayEvent::SendError(e) => {
                                                tracing::error!("Relay send error - {} - {}", &url, &e);
                                            }
                                            RelayEvent::RelayInformation(info) => {
                                                tracing::trace!("Relay info - {} - {:?}", &url, &info);
                                                if let Err(e) = handle_relay_info(&mut output, backend, url, info).await {
                                                    tracing::error!("{}", e);
                                                }
                                            }
                                            RelayEvent::Timeout(subscription_id) => {
                                                if let Err(e) = handle_eose(&mut output, keys, backend, url, subscription_id).await {
                                                    // depending on the error, restart backend?
                                                    tracing::error!("{}", e);
                                                }
                                            }
                                            RelayEvent::RelayMessage(message) => {
                                                if let Err(e) = handle_relay_message(&mut output, &keys, backend, tasks_tx, url, message).await{
                                                    // depending on the error, restart backend?
                                                    tracing::error!("{}", e);
                                                }
                                            }
                                            RelayEvent::SentSubscription(sub_id) => {
                                                tracing::debug!("Sent subscription to {} - id: {}", url, sub_id);
                                            }
                                            RelayEvent::SentCount(sub_id) => {
                                                tracing::debug!("Sent count to {} - id: {}", url, sub_id);
                                            }
                                            RelayEvent::SentEvent(event_hash) => {
                                                tracing::debug!("Sent event to {} - hash: {}", url, event_hash);
                                            }
                                        };
                                    } else {
                                        tracing::info!("Nostr notification failed");
                                    }
                                },
                                task_result = tasks_rx.recv() => {
                                    if let Some(task_result) = task_result {
                                        if let Err(e) = handle_task_result(&mut output, &keys, backend, task_result).await{
                                            // depending on the error, restart backend?
                                            tracing::error!("{}", e);
                                        }
                                    } else {
                                        tracing::trace!("Tasks channel closed");
                                    }
                                }
                            };
                        }
                    }
                }
            }
        }
    })
}

async fn handle_relay_info(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    backend: &mut BackendState,
    url: Url,
    info: ns_client::RelayInformation,
) -> Result<(), Error> {
    let db_relay = DbRelay::fetch_by_url(&backend.db_client.pool, &url)
        .await?
        .map(|mut db_relay| {
            db_relay.information = Some(info);
            db_relay
        });

    if let Some(db_relay) = db_relay {
        _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
    }

    Ok(())
}

async fn handle_task_result(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    _keys: &Keys,
    backend: &mut BackendState,
    result: Result<TaskOutput, Error>,
) -> Result<(), Error> {
    let task_result = result?;
    let pool = &backend.db_client.pool;
    let cache_pool = &backend.db_client.cache_pool;
    match task_result {
        TaskOutput::Ntp(ntp_time) => {
            tracing::info!("NTP time: {}", ntp_time);
            UserConfig::update_ntp_offset(pool, ntp_time).await?;
            _ = output.send(BackendEvent::SyncedWithNtpServer).await;
        }
        TaskOutput::ImageDownloaded(image) => {
            ImageDownloaded::insert(cache_pool, &image).await?;
            _ = output.send(BackendEvent::ImageDownloaded(image)).await;
        }
        TaskOutput::LatestVersion(version) => {
            _ = output.send(BackendEvent::LatestVersion(version)).await;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    // --- REQWEST ---
    LatestVersion(String),
    FetchingLatestVersion,
    DownloadingImage {
        kind: ImageKind,
        event_hash: EventId,
    },
    ImageDownloaded(ImageDownloaded),

    // ---  ---
    GotKeys(Keys),
    GotChatMessages((DbContact, Vec<ChatMessage>)),
    GotRelayResponses {
        chat_message: ChatMessage,
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotRelayResponsesUserProfile {
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotRelayResponsesContactList {
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotContacts(Vec<DbContact>),
    RelayCreated(DbRelay),
    RelayUpdated(DbRelay),
    RelayDeleted(Url),
    GotRelays(Vec<DbRelay>),
    ContactCreated(DbContact),
    ContactUpdated(DbContact),
    ContactDeleted(DbContact),
    OtherKindEventInserted(DbEvent),
    GotUserProfileCache(Option<ProfileCache>),
    FileContactsImported(Vec<DbContact>),
    UserProfilePictureUpdated(PathBuf),
    UserBannerPictureUpdated(PathBuf),
    UpdatedMetadata(XOnlyPublicKey),
    GotAllMessages(Vec<DbEvent>),
    GotSingleContact(XOnlyPublicKey, Option<DbContact>),
    GotChatInfo(DbContact, ChatInfo),
    GotRelayStatusList(ns_client::RelayStatusList),
    CacheFileRemoved((ProfileCache, ImageKind)),
    RelayDocument(DbRelay),
    ChannelConfirmed(ChannelCache),
    ChannelUpdated(ChannelCache),

    // --- Config ---
    SyncedWithNtpServer,
    Connected(BackEndConnection),
    FinishedPreparing,
    LoginSuccess,
    FirstLoginSuccess,
    FailedToStartClient,
    CreateAccountSuccess,
    LogoutSuccess,

    // --- Specific Events ---
    ReceivedDM {
        relay_url: Url,
        db_contact: DbContact,
        chat_message: ChatMessage,
    },
    ReceivedContactList,
    UpdatedContactMetadata {
        relay_url: Url,
        db_contact: DbContact,
    },
    // --- Confirmed Events ---
    ConfirmedDM((DbContact, ChatMessage)),
    ConfirmedContactList(DbEvent),
    ConfirmedMetadata {
        db_event: DbEvent,
        is_user: bool,
    },

    // --- Pending Events ---
    PendingDM((DbContact, ChatMessage)),

    // --- RFD ---
    RFDPickedFile(PathBuf),
    RFDPickError(String),
    RFDCancelPick,
    RFDSavedFile(PathBuf),

    SearchingForChannels,
    EOSESearchChannels(Url),
    SearchChannelResult(ChannelResult),
    SearchChannelMetaUpdate(PrefixedId, ChannelCache),
    SearchChannelMessage(PrefixedId, XOnlyPublicKey),
    EOSESearchChannelsDetails(Url, PrefixedId),
    LoadingChannelDetails(Url, PrefixedId),
    GotRelay(Option<DbRelay>),
    RelayError(Url, String),
    GotNipsData(Vec<NipData>),
}

#[derive(Debug, Clone)]
pub enum ToBackend {
    Logout,
    FetchLatestVersion,
    QueryFirstLogin,
    PrepareClient,

    FetchRelayResponsesChatMsg(ChatMessage),
    FetchRelayResponsesUserProfile,
    FetchRelayResponsesContactList,
    FetchRelays,
    FetchRelay(Url),
    FetchMessages(DbContact),
    AddRelay(Url),
    DeleteRelay(Url),
    ToggleRelayRead(DbRelay),
    ToggleRelayWrite(DbRelay),
    GetRelayInformation,
    FetchNipsData,

    FetchContacts,
    AddContact(DbContact),
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    ImportContacts(Vec<DbContact>, bool),

    GetUserProfileMeta,
    UpdateUserProfileMeta(Metadata),
    FetchAllMessageEvents,
    ExportMessages(Vec<DbEvent>),
    ExportContacts,
    FetchChatInfo(DbContact),
    FetchContactWithMetadata(XOnlyPublicKey),
    RefreshContactsProfile,
    SendDM((DbContact, String)),
    CreateChannel,
    FetchMoreMessages((DbContact, ChatMessage)),
    ChooseFile(Option<FileFilter>),
    LoginWithSK(Keys),
    CreateAccount(BasicProfile),
    FindChannels(String),
    FetchKeys,
    DownloadImage {
        image_url: String,
        kind: ImageKind,
        identifier: String,
        event_hash: EventId,
    },
}

pub async fn process_message(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    backend: &mut BackendState,
    task_tx: &tokio::sync::mpsc::Sender<Result<TaskOutput, Error>>,
    message: ToBackend,
) -> Result<(), Error> {
    tracing::debug!("Processing message: {:?}", message);
    match message {
        // ---- CONFIG ----
        ToBackend::LoginWithSK(_) => {
            unreachable!("Login with sk client should be sent only once")
        }
        ToBackend::Logout => {
            unreachable!("Logout should be processed outside here")
        }
        ToBackend::CreateAccount(profile) => {
            let profile_meta: Metadata = profile.into();
            backend.new_profile_event(&keys, &profile_meta).await?;
            // _ = output
            //     .send(BackendEvent::PendingMetadata(pending_event))
            //     .await;
        }
        // --- RFD ---
        ToBackend::ExportMessages(messages) => {
            let ns_events: Result<Vec<_>, _> = messages.iter().map(|m| m.to_ns_event()).collect();
            let ns_events = ns_events?; // Unwrap the Result, propagating any errors.
            match save_file(&ns_events, "json").await {
                Ok(event) => {
                    _ = output.send(event).await;
                }
                Err(e) => {
                    tracing::error!("Failed to export messages: {}", e);
                    _ = output.send(BackendEvent::RFDPickError(e.to_string())).await;
                }
            }
        }
        ToBackend::ExportContacts => {
            let pending_event = backend.new_contact_list_event(keys).await?;
            match save_file(pending_event.ns_event(), "json").await {
                Ok(event) => {
                    _ = output.send(event).await;
                }
                Err(e) => {
                    tracing::error!("Failed to export contacts: {}", e);
                    _ = output.send(BackendEvent::RFDPickError(e.to_string())).await;
                }
            }
        }
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
            match rfd_instance.pick_file().await {
                Some(handle) => {
                    _ = output
                        .send(BackendEvent::RFDPickedFile(handle.path().to_owned()))
                        .await;
                }
                None => {
                    _ = output.send(BackendEvent::RFDCancelPick).await;
                }
            }
        }
        // ---- REQWEST ----
        ToBackend::FetchLatestVersion => {
            let task_tx_1 = task_tx.clone();
            let req_client_1 = backend.req_client.clone();
            tokio::spawn(async move {
                let result = fetch_latest_version(req_client_1)
                    .await
                    .map(TaskOutput::LatestVersion)
                    .map_err(|e| e.into());
                if let Err(e) = task_tx_1.send(result).await {
                    tracing::error!("Error sending latest version to backend: {}", e);
                }
            });
            _ = output.send(BackendEvent::FetchingLatestVersion).await;
        }
        ToBackend::DownloadImage {
            image_url,
            identifier,
            kind,
            event_hash,
        } => {
            match ImageDownloaded::fetch(&backend.db_client.cache_pool, &event_hash, kind).await? {
                Some(image) => {
                    _ = output.send(BackendEvent::ImageDownloaded(image)).await;
                }
                None => {
                    let task_tx_1 = task_tx.clone();
                    let identifier_1 = identifier.clone();
                    let image_url_1 = image_url.to_string();
                    tokio::spawn(async move {
                        let result = download_image(&image_url_1, &event_hash, &identifier_1, kind)
                            .await
                            .map(TaskOutput::ImageDownloaded)
                            .map_err(|e| e.into());
                        if let Err(e) = task_tx_1.send(result).await {
                            tracing::error!("Error sending image downloaded event: {}", e);
                        }
                    });
                    _ = output
                        .send(BackendEvent::DownloadingImage { kind, event_hash })
                        .await;
                }
            }
        }
        // -----------
        ToBackend::FetchNipsData => {
            _ = output
                .send(BackendEvent::GotNipsData(backend.nips_data.clone()))
                .await;
        }
        ToBackend::FetchRelay(url) => {
            let relay = DbRelay::fetch_by_url(&backend.db_client.pool, &url).await?;
            _ = output.send(BackendEvent::GotRelay(relay)).await;
        }
        ToBackend::FetchRelays => {
            let relays = DbRelay::fetch(&backend.db_client.pool).await?;
            for r in &relays {
                let _ = backend.nostr.relay_info(&r.url);
            }
            _ = output.send(BackendEvent::GotRelays(relays)).await;
        }
        ToBackend::AddRelay(url) => {
            backend.nostr.add_relay(url.as_str())?;
            let db_relay = DbRelay::insert(&backend.db_client.pool, &url).await?;
            _ = output.send(BackendEvent::RelayCreated(db_relay)).await;
        }
        ToBackend::DeleteRelay(url) => {
            backend.nostr.remove_relay(url.as_str())?;
            DbRelay::delete(&backend.db_client.pool, &url).await?;
            _ = output.send(BackendEvent::RelayDeleted(url)).await;
        }
        ToBackend::ToggleRelayRead(mut db_relay) => {
            db_relay.read = !db_relay.read;
            backend
                .nostr
                .toggle_read_for(&db_relay.url, db_relay.read)?;
            DbRelay::update(&backend.db_client.pool, &db_relay).await?;
            _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
        }
        ToBackend::ToggleRelayWrite(mut db_relay) => {
            db_relay.write = !db_relay.write;
            backend
                .nostr
                .toggle_write_for(&db_relay.url, db_relay.write)?;
            DbRelay::update(&backend.db_client.pool, &db_relay).await?;
            _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
        }
        ToBackend::FetchRelayResponsesUserProfile => {
            let pool = &backend.db_client.pool;
            if let Some(profile_event) =
                DbEvent::fetch_last_kind_pubkey(pool, Kind::Metadata, &keys.public_key()).await?
            {
                let all_relays = DbRelay::fetch(pool).await?;
                let responses =
                    DbRelayResponse::fetch_by_event(pool, profile_event.event_id).await?;
                _ = output
                    .send(BackendEvent::GotRelayResponsesUserProfile {
                        responses,
                        all_relays,
                    })
                    .await;
            }
        }
        ToBackend::FetchRelayResponsesContactList => {
            let pool = &backend.db_client.pool;
            if let Some(profile_event) =
                DbEvent::fetch_last_kind_pubkey(pool, Kind::ContactList, &keys.public_key()).await?
            {
                let all_relays = DbRelay::fetch(pool).await?;
                let responses =
                    DbRelayResponse::fetch_by_event(pool, profile_event.event_id).await?;
                _ = output
                    .send(BackendEvent::GotRelayResponsesContactList {
                        responses,
                        all_relays,
                    })
                    .await;
            }
        }
        ToBackend::FetchRelayResponsesChatMsg(chat_message) => {
            let pool = &backend.db_client.pool;
            if let Some(db_message) = DbMessage::fetch_one(pool, chat_message.msg_id).await? {
                if let Some(confirmation) = db_message.confirmation_info {
                    let all_relays = DbRelay::fetch(pool).await?;
                    let responses =
                        DbRelayResponse::fetch_by_event(pool, confirmation.event_id).await?;
                    _ = output
                        .send(BackendEvent::GotRelayResponses {
                            responses,
                            all_relays,
                            chat_message,
                        })
                        .await;
                }
            }
        }
        ToBackend::FetchKeys => {
            _ = output.send(BackendEvent::GotKeys(keys.to_owned())).await;
        }
        ToBackend::FindChannels(search_term) => {
            backend.nostr.subscribe_eose(
                &SubscriptionType::SearchChannels.id(),
                vec![channel_search_filter(&search_term)],
                Some(Duration::from_secs(10)),
            )?;
            _ = output.send(BackendEvent::SearchingForChannels).await;
        }
        ToBackend::UpdateUserProfileMeta(profile_meta) => {
            backend.new_profile_event(keys, &profile_meta).await?;
            // _ = output
            //     .send(BackendEvent::PendingMetadata(pending_event))
            //     .await;
        }
        ToBackend::QueryFirstLogin => {
            let pool = &backend.db_client.pool;
            if UserConfig::query_has_logged_in(pool).await? {
                prepare_client(keys, backend).await?;
                _ = output.send(BackendEvent::FinishedPreparing).await;
            } else {
                _ = output.send(BackendEvent::FirstLoginSuccess).await;
            }
        }
        ToBackend::PrepareClient => {
            prepare_client(keys, backend).await?;
            _ = output.send(BackendEvent::FinishedPreparing).await;
        }
        ToBackend::FetchMoreMessages((db_contact, first_message)) => {
            let pool = &backend.db_client.pool;
            let msgs = DbMessage::fetch_more(pool, db_contact.pubkey(), first_message).await?;
            match msgs.is_empty() {
                // update nostr subscriber
                true => (),
                false => {
                    let mut chat_messages = vec![];
                    tracing::debug!("Decrypting messages");
                    for db_message in &msgs {
                        if let Some(db_event) =
                            DbEvent::fetch_hash(pool, &db_message.event_hash).await?
                        {
                            let tag_info = TagInfo::from_db_event(&db_event)?;
                            let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
                            let chat_message =
                                ChatMessage::new(&db_message, &db_contact, &decrypted_content)?;
                            chat_messages.push(chat_message);
                        }
                    }
                    _ = output
                        .send(BackendEvent::GotChatMessages((db_contact, chat_messages)))
                        .await;
                }
            }
        }
        ToBackend::FetchContactWithMetadata(pubkey) => {
            let req = DbContact::fetch_one(
                &backend.db_client.pool,
                &backend.db_client.cache_pool,
                &pubkey,
            )
            .await?;
            _ = output
                .send(BackendEvent::GotSingleContact(pubkey, req))
                .await;
        }

        ToBackend::FetchChatInfo(db_contact) => {
            let pool = &backend.db_client.pool;
            if let Some(db_message) = DbMessage::fetch_chat_last(pool, &db_contact.pubkey()).await?
            {
                if let Some(db_event) = DbEvent::fetch_hash(pool, &db_message.event_hash).await? {
                    let unseen_messages =
                        DbMessage::fetch_unseen_chat_count(pool, &db_contact.pubkey()).await?;
                    let tag_info = TagInfo::from_db_event(&db_event)?;
                    let decrypted_content = db_message.decrypt_message(&keys, &tag_info)?;
                    _ = output
                        .send(BackendEvent::GotChatInfo(
                            db_contact,
                            ChatInfo {
                                unseen_messages,
                                last_message: decrypted_content,
                                last_message_time: Some(db_message.display_time()),
                            },
                        ))
                        .await;
                }
            }
        }
        ToBackend::FetchAllMessageEvents => {
            let messages =
                DbEvent::fetch_kind(&backend.db_client.pool, Kind::EncryptedDirectMessage).await?;
            _ = output.send(BackendEvent::GotAllMessages(messages)).await;
        }
        ToBackend::GetUserProfileMeta => {
            let cache = ProfileCache::fetch_by_public_key(
                &backend.db_client.cache_pool,
                &keys.public_key(),
            )
            .await?;
            _ = output.send(BackendEvent::GotUserProfileCache(cache)).await;
        }
        ToBackend::ImportContacts(db_contacts, _is_replace) => {
            for _db_contact in &db_contacts {
                // check if contact already exists
                // if exists and replace just upsert
                // else none
                todo!();
            }
            backend.new_contact_list_event(keys).await?;
            _ = output
                .send(BackendEvent::FileContactsImported(db_contacts))
                .await;
        }
        ToBackend::AddContact(db_contact) => {
            // Check if the contact is the same as the user
            if &keys.public_key() == db_contact.pubkey() {
                return Err(Error::SameContactInsert);
            }
            // add or update basic
            DbContact::upsert_contact(&backend.db_client.pool, &db_contact).await?;
            backend.new_contact_list_event(keys).await?;
            _ = output.send(BackendEvent::ContactCreated(db_contact)).await;
        }
        ToBackend::UpdateContact(db_contact) => {
            if &keys.public_key() == db_contact.pubkey() {
                return Err(Error::SameContactUpdate);
            }
            DbContact::update(&backend.db_client.pool, &db_contact).await?;
            backend.new_contact_list_event(keys).await?;
            _ = output.send(BackendEvent::ContactUpdated(db_contact)).await;
        }
        ToBackend::DeleteContact(db_contact) => {
            DbContact::delete(&backend.db_client.pool, &db_contact).await?;
            backend.new_contact_list_event(keys).await?;
            _ = output.send(BackendEvent::ContactDeleted(db_contact)).await;
        }
        ToBackend::FetchContacts => {
            let contacts =
                DbContact::fetch(&backend.db_client.pool, &backend.db_client.cache_pool).await?;
            _ = output.send(BackendEvent::GotContacts(contacts)).await;
        }

        ToBackend::FetchMessages(db_contact) => {
            let pool = &backend.db_client.pool;
            let db_messages = DbMessage::fetch_chat(pool, db_contact.pubkey()).await?;

            // Maybe the message is only seen when scrolling?
            tracing::debug!("Updating unseen messages to marked as seen");
            DbMessage::reset_unseen(pool, db_contact.pubkey()).await?;

            // Maybe a spawned task?
            tracing::debug!("Decrypting messages");
            let mut chat_messages = vec![];
            for db_message in &db_messages {
                if let Some(db_event) = DbEvent::fetch_hash(pool, &db_message.event_hash).await? {
                    let tag_info = TagInfo::from_db_event(&db_event)?;
                    let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
                    let chat_message =
                        ChatMessage::new(&db_message, &db_contact, &decrypted_content)?;
                    chat_messages.push(chat_message);
                }
            }

            _ = output
                .send(BackendEvent::GotChatMessages((db_contact, chat_messages)))
                .await;
        }

        ToBackend::GetRelayInformation => {
            backend.nostr.relays_info()?;
        }

        ToBackend::RefreshContactsProfile => {
            // subscribe_eose contact_list_metadata
            todo!();
        }

        ToBackend::CreateChannel => {
            todo!()
        }

        ToBackend::SendDM((db_contact, raw_content)) => {
            // create a pending event and await confirmation of relays
            let pending_event = backend.new_dm(keys, &db_contact, &raw_content).await?;
            let pending_msg = DbMessage::insert_pending(
                &backend.db_client.pool,
                pending_event,
                &db_contact.pubkey(),
                true,
            )
            .await?;

            let chat_message = ChatMessage::new(&pending_msg, &db_contact, &raw_content)?;

            _ = output
                .send(BackendEvent::PendingDM((db_contact, chat_message)))
                .await;
        }
    }

    Ok(())
}

async fn handle_metadata_event(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<(), Error> {
    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::trace!("{:?}", db_event);

    let rows_changed = ProfileCache::insert(cache_pool, db_event).await?;

    if rows_changed == 0 {
        tracing::debug!("Cache already up to date");
    }

    _ = output
        .send(BackendEvent::UpdatedMetadata(db_event.pubkey))
        .await;

    Ok(())
}

async fn handle_channel_creation(
    _output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    _cache_pool: &SqlitePool,
    _db_event: &DbEvent,
) -> Result<(), Error> {
    todo!()
    // let channel_cache = ChannelCache::fetch_insert(cache_pool, db_event).await?;
    // _ = output
    //     .send(BackendEvent::ChannelConfirmed(channel_cache))
    //     .await;
    // Ok(())
}

async fn handle_channel_update(
    _output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    _cache_pool: &SqlitePool,
    _db_event: &DbEvent,
) -> Result<(), Error> {
    todo!()
    // ChannelCache::update(cache_pool, db_event).await?;
    // let channel_cache = ChannelCache::update(cache_pool, db_event).await?;
    // _ = output
    //     .send(BackendEvent::ChannelUpdated(channel_cache))
    //     .await;
    // Ok(())
}

async fn handle_channel_message(
    _output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    _cache_pool: &SqlitePool,
    _db_event: &DbEvent,
) -> Result<(), Error> {
    todo!()
}

pub async fn handle_recommend_relay(db_event: DbEvent) -> Result<(), Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&db_event);
    Ok(())
}

pub async fn _create_channel(_client: &RelayPool) -> Result<BackendEvent, Error> {
    // tracing::debug!("create_channel");
    // let metadata = Metadata::new()
    //     .about("Channel about cars")
    //     .display_name("Best Cars")
    //     .name("Best Cars")
    //     .banner(Url::from_str("https://picsum.photos/seed/picsum/800/300")?)
    //     .picture(Url::from_str("https://picsum.photos/seed/picsum/200/300")?)
    //     .website(Url::from_str("https://picsum.photos/seed/picsum/200/300")?);

    // let event_id = client.new_channel(metadata).await?;

    // Ok(BackendEvent::ChannelCreated(event_id))
    todo!()
}

pub fn add_relays_and_connect(
    backend: &mut BackendState,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
) -> Result<(), Error> {
    tracing::info!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        let opts = ns_client::RelayOptions::new(r.read, r.write);
        match backend.nostr.add_relay_with_opts(&r.url.to_string(), opts) {
            Ok(_) => tracing::debug!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    let last_event_tt: u64 = last_event
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.created_at - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0);
    tracing::info!("last event timestamp: {}", last_event_tt);

    backend.nostr.subscribe_eose(
        &SubscriptionType::ContactList.id(),
        vec![contact_list_filter(keys.public_key(), last_event_tt)],
        None,
    )?;
    backend.nostr.subscribe_eose(
        &SubscriptionType::UserMetadata.id(),
        vec![user_metadata(keys.public_key(), last_event_tt)],
        None,
    )?;
    backend.nostr.subscribe_id(
        &SubscriptionType::Messages.id(),
        messages_filter(keys.public_key(), last_event_tt),
        None,
    )?;

    Ok(())
}

async fn prepare_client(keys: &Keys, backend: &mut BackendState) -> Result<(), Error> {
    let pool = &backend.db_client.pool;
    let relays = DbRelay::fetch(pool).await?;
    let last_event = DbEvent::fetch_last_kind(pool, Kind::EncryptedDirectMessage).await?;

    UserConfig::store_first_login(pool).await?;

    add_relays_and_connect(backend, &keys, &relays, last_event)?;

    if let Some(profile) = backend.create_account.take() {
        let profile_meta: Metadata = profile.into();
        backend.new_profile_event(&keys, &profile_meta).await?;
    }

    Ok(())
}

async fn save_with_extension<T: Serialize>(
    file_handle: rfd::FileHandle,
    extension: &str,
    data: &T,
) -> Result<PathBuf, Error> {
    let mut path = file_handle.path().to_path_buf();
    path.set_extension(extension);
    let json = serde_json::to_vec(data)?;
    tokio::fs::write(&path, json).await?;
    Ok(path)
}

async fn save_file<T: Serialize>(data: &T, extension: &str) -> Result<BackendEvent, Error> {
    let rfd_instance = AsyncFileDialog::new().set_directory("/");
    let file_handle = rfd_instance.save_file().await;
    match file_handle {
        Some(file_handle) => {
            let path = save_with_extension(file_handle, extension, data).await?;
            Ok(BackendEvent::RFDSavedFile(path))
        }
        None => {
            tracing::debug!("No file selected for exporting.");
            Ok(BackendEvent::RFDCancelPick)
        }
    }
}

const BACKEND_CHANNEL_SIZE: usize = 1024;
