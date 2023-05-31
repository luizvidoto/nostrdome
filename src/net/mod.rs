use crate::components::async_file_importer::FileFilter;
use crate::components::chat_contact::ChatInfo;
use crate::db::ChannelCache;
use crate::db::DbContact;
use crate::db::DbEvent;
use crate::db::DbMessage;
use crate::db::DbRelay;
use crate::db::DbRelayResponse;
use crate::db::ProfileCache;
use crate::db::UserConfig;
use crate::net::operations::builder::build_contact_list_event;
use crate::net::operations::builder::build_dm;
use crate::net::operations::builder::build_profile_event;
use crate::net::reqwest_client::fetch_latest_version;
use crate::types::ChatMessage;
use crate::views::login::BasicProfile;
use crate::Error;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::db::Database;
use crate::net::ntp::spawn_ntp_request;

use futures_util::SinkExt;
use iced::subscription;
use iced::Subscription;
use nostr::Filter;
use nostr::Keys;
use nostr::Kind;
use nostr::RelayMessage;

pub(crate) mod ntp;
pub(crate) mod operations;
pub(crate) mod reqwest_client;

use nostr::secp256k1::XOnlyPublicKey;
use nostr::SubscriptionId;
use nostr::Timestamp;
use ns_client::NotificationEvent;
use ns_client::RelayPool;

pub(crate) use reqwest_client::{image_filename, sized_image, ImageKind, ImageSize};
use rfd::AsyncFileDialog;
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use url::Url;

use self::operations::contact_list::handle_contact_list;
use self::operations::direct_message::handle_dm;
use self::operations::event::confirmed_event;
use self::operations::event::insert_pending;
use self::reqwest_client::download_image;
use self::reqwest_client::ImageDownloaded;

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
        nostr: NostrState,
    },
}

pub struct NostrState {
    pub client: RelayPool,
    pub notifications: broadcast::Receiver<NotificationEvent>,
}
impl NostrState {
    pub fn new(_keys: &Keys) -> Self {
        let client = RelayPool::new();
        let notifications = client.notifications();
        Self {
            client,
            notifications,
        }
    }
    pub async fn logout(&self) -> Result<(), Error> {
        Ok(self.client.shutdown()?)
    }
}
pub struct BackendState {
    pub db_client: Database,
    pub req_client: reqwest::Client,
    pub subscriptions: HashMap<SubscriptionId, SubscriptionType>,
}
impl BackendState {
    pub fn new(db_client: Database, req_client: reqwest::Client) -> Self {
        let mut subscriptions = HashMap::new();
        for sub_type in SubscriptionType::ALL.iter() {
            subscriptions.insert(SubscriptionId::new(sub_type.to_string()), *sub_type);
        }
        Self {
            db_client,
            req_client,
            subscriptions,
        }
    }

    pub async fn logout(&self) {
        tracing::info!("Database Logging out");
        self.db_client.pool.close().await;
        self.db_client.cache_pool.close().await;
    }
}

pub enum TaskOutput {
    Ntp(u64),
    LatestVersion(String),
    ImageDownloaded(ImageDownloaded),
}

// if UserConfig::query_has_logged_in(pool).await? {
//     //TODO: exatamente igual o de baixo
//     let relays = DbRelay::fetch(pool).await?;
//     let last_event =
//         DbEvent::fetch_last_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
//     UserConfig::store_first_login(pool).await?;
//     add_relays_and_connect(nostr, &keys, &relays, last_event)?;
//     let _ = output.send(BackendEvent::LoginSuccess).await;
// } else {
//     let _ = output.send(BackendEvent::FirstLoginSuccess).await;
// }

async fn handle_relay_message(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    nostr: &RelayPool,
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
            if status == false {
                let db_relay = DbRelay::update_with_error(pool, &url, &error_msg).await?;
                let _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
                return Ok(());
            }
            tracing::debug!("Relay message: Ok");

            let mut db_event = DbEvent::fetch_one(pool, &event_hash)
                .await?
                .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;

            if db_event.relay_url.is_none() {
                db_event = DbEvent::confirm_event(pool, &url, db_event).await?;
            }

            DbRelayResponse::insert_ok(pool, &url, &db_event).await?;

            match db_event.kind {
                nostr::Kind::ContactList => {
                    let _ = output
                        .send(BackendEvent::ConfirmedContactList(db_event.to_owned()))
                        .await;
                }
                nostr::Kind::Metadata => {
                    let is_user = db_event.pubkey == keys.public_key();
                    let _ = output
                        .send(BackendEvent::ConfirmedMetadata {
                            db_event: db_event.to_owned(),
                            is_user,
                        })
                        .await;
                }
                nostr::Kind::EncryptedDirectMessage => {
                    let relay_url = db_event
                        .relay_url
                        .as_ref()
                        .ok_or(Error::NotConfirmedEvent(db_event.event_hash.to_owned()))?;

                    if let Some(db_message) =
                        DbMessage::fetch_by_event(pool, db_event.event_id()?).await?
                    {
                        let db_message =
                            DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
                        if let Some(db_contact) =
                            DbContact::fetch_one(pool, cache_pool, &db_message.contact_chat())
                                .await?
                        {
                            let chat_message =
                                ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
                            let _ = output
                                .send(BackendEvent::ConfirmedDM((db_contact, chat_message)))
                                .await;
                            return Ok(());
                        } else {
                            return Err(Error::ContactNotFound(
                                db_message.contact_chat().to_owned(),
                            ));
                        }
                    }
                    return Err(Error::EventWithoutMessage(db_event.event_id()?));
                }
                _ => {
                    return Err(Error::NotSubscribedToKind(db_event.kind.clone()));
                }
            }
        }
        RelayMessage::EndOfStoredEvents(subscription_id) => {
            if let Some(sub_type) = backend.subscriptions.get(&subscription_id) {
                match sub_type {
                    SubscriptionType::ContactList => {
                        let list = DbContact::fetch(pool, cache_pool).await?;
                        let id =
                            SubscriptionId::new(SubscriptionType::ContactListMetadata.to_string());
                        let filters = vec![contact_list_metadata(&list)];
                        nostr.relay_subscribe_eose(&url, &id, filters)?;
                    }
                    // SubscriptionType::Messages => todo!(),
                    // SubscriptionType::ContactListMetadata => todo!(),
                    // SubscriptionType::Channel => todo!(),
                    // SubscriptionType::ChannelMetadata => todo!(),
                    _other => (),
                }
            }
        }
        RelayMessage::Event {
            subscription_id: _,
            event: ns_event,
        } => {
            match ns_event.kind {
                nostr::Kind::ContactList => {
                    // contact_list is validating differently
                    handle_contact_list(output, keys, pool, &url, *ns_event).await?;
                }
                other => {
                    if let Some(db_event) = confirmed_event(pool, &url, *ns_event).await? {
                        match other {
                            nostr::Kind::Metadata => {
                                handle_metadata_event(output, cache_pool, &db_event).await?;
                            }
                            nostr::Kind::EncryptedDirectMessage => {
                                handle_dm(output, pool, cache_pool, keys, &url, &db_event).await?;
                            }
                            nostr::Kind::RecommendRelay => {
                                handle_recommend_relay(db_event).await?;
                            }
                            nostr::Kind::ChannelCreation => {
                                handle_channel_creation(output, cache_pool, &db_event).await?;
                            }
                            nostr::Kind::ChannelMetadata => {
                                handle_channel_update(output, cache_pool, &db_event).await?;
                            }
                            _other_kind => {
                                let _ = output
                                    .send(BackendEvent::OtherKindEventInserted(db_event))
                                    .await;
                            }
                        }
                    }
                }
            }
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

async fn get_clients(keys: &Keys) -> Result<ClientState, Error> {
    let db_client = Database::new(&keys.public_key().to_string()).await?;
    let (tasks_tx, tasks_rx) = tokio::sync::mpsc::channel(100);
    let nostr = NostrState::new(&keys);
    let req_client = reqwest::Client::new();
    let backend = BackendState::new(db_client, req_client);

    spawn_ntp_request(tasks_tx.clone());

    Ok(ClientState::Connected {
        tasks_rx,
        tasks_tx,
        keys: keys.to_owned(),
        backend,
        nostr,
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
                    let _ = output
                        .send(BackendEvent::Connected(BackEndConnection::new(sender)))
                        .await;
                }
                State::Ready(receiver) => {
                    match &mut client_state {
                        ClientState::Empty => {
                            if let Some(input) = receiver.recv().await {
                                match input {
                                    ToBackend::LoginWithSK(keys) => {
                                        match get_clients(&keys).await {
                                            Ok(state) => {
                                                client_state = state;
                                                let _ =
                                                    output.send(BackendEvent::LoginSuccess).await;
                                            }
                                            Err(e) => {
                                                tracing::error!("{}", e);
                                                let _ = output
                                                    .send(BackendEvent::FailedToStartClient)
                                                    .await;
                                            }
                                        }
                                    }
                                    // TODO: do something with profile
                                    // create profile to send later to a relay when connected
                                    ToBackend::CreateAccount(profile) => {
                                        let keys = Keys::generate();
                                        match get_clients(&keys).await {
                                            Ok(state) => {
                                                client_state = state;
                                                let _ = output
                                                    .send(BackendEvent::CreateAccountSuccess)
                                                    .await;
                                            }
                                            Err(e) => {
                                                tracing::error!("{}", e);
                                                let _ = output
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
                            nostr,
                            keys,
                        } => {
                            tokio::select! {
                                message = receiver.recv() => {
                                    tracing::debug!("Received message from frontend");
                                    if let Some(message) = message {
                                        if let ToBackend::Logout = message {
                                            backend.logout().await;
                                            let _ = nostr.logout().await;

                                            let _ = output.send(BackendEvent::LogoutSuccess).await;
                                            state = State::Start;
                                            client_state = ClientState::Empty;

                                        } else {
                                            if let Err(e) = process_message(&mut output, &keys, &nostr.client, backend, tasks_tx, message).await {
                                                // depending on the error, restart backend?
                                                tracing::error!("{}", e);
                                            }
                                        }
                                    } else {
                                        tracing::info!("Front to backend channel closed");
                                        backend.logout().await;
                                        let _ = nostr.logout().await;

                                        let _ = output.send(BackendEvent::LogoutSuccess).await;
                                        state = State::Start;
                                        client_state = ClientState::Empty;
                                    }
                                }
                                notification = nostr.notifications.recv() => {
                                    tracing::debug!("Received notification from nostr");
                                    if let Ok(notification) = notification{
                                        match notification {
                                            NotificationEvent::RelayTerminated(url) => {
                                                tracing::debug!("Relay terminated - {}", url);
                                            }
                                            NotificationEvent::RelayMessage(url, message) => {
                                                if let Err(e) = handle_relay_message(&mut output, &keys, &nostr.client, backend, tasks_tx, url, message).await{
                                                    // depending on the error, restart backend?
                                                    tracing::error!("{}", e);
                                                }
                                            }
                                            NotificationEvent::SentSubscription(url, sub_id) => {
                                                tracing::debug!("Sent subscription to {} - id: {}", url, sub_id);
                                            }
                                            NotificationEvent::SentEvent(url, event_hash) => {
                                                tracing::debug!("Sent event to {} - hash: {}", url, event_hash);
                                            }
                                        };
                                    } else {
                                        tracing::info!("Nostr notification failed");
                                    }
                                },
                                task_result = tasks_rx.recv() => {
                                    if let Some(task_result) = task_result {
                                        if let Err(e) = handle_task_result(&mut output, &keys, &nostr.client, backend, task_result).await{
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

async fn handle_task_result(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    _nostr: &RelayPool,
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
            let _ = output.send(BackendEvent::SyncedWithNtpServer).await;
        }
        TaskOutput::ImageDownloaded(img) => {
            ProfileCache::image_downloaded(cache_pool, &img).await?;
            match keys.public_key() == img.public_key {
                true => match img.kind {
                    ImageKind::Profile => {
                        let _ = output
                            .send(BackendEvent::UserProfilePictureUpdated(img.path))
                            .await;
                    }
                    ImageKind::Banner => {
                        let _ = output
                            .send(BackendEvent::UserBannerPictureUpdated(img.path))
                            .await;
                    }
                },
                false => {
                    if let Some(db_contact) =
                        DbContact::fetch_one(pool, cache_pool, &img.public_key).await?
                    {
                        let _ = output.send(BackendEvent::ContactUpdated(db_contact)).await;
                    } else {
                        tracing::warn!(
                            "Image downloaded for contact not found in database: {}",
                            &img.public_key
                        );
                        return Err(Error::ContactNotFound(img.public_key));
                    }
                }
            }
        }
        TaskOutput::LatestVersion(version) => {
            let _ = output.send(BackendEvent::LatestVersion(version)).await;
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
        public_key: XOnlyPublicKey,
    },
    // ---  ---
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
    RelayDeleted(DbRelay),
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
    // GotDbEvent(Option<DbEvent>),
    GotSingleContact((XOnlyPublicKey, Option<DbContact>)),
    GotChatInfo((DbContact, Option<ChatInfo>)),
    ChannelCreated(ChannelCache),
    GotRelayStatusList(ns_client::RelayStatusList),
    CacheFileRemoved((ProfileCache, ImageKind)),
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
        relay_url: nostr::Url,
        db_contact: DbContact,
        chat_message: ChatMessage,
    },
    ReceivedContactList {
        relay_url: nostr::Url,
        contact_list: Vec<DbContact>,
    },
    UpdatedContactMetadata {
        relay_url: nostr::Url,
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
    PendingMetadata(DbEvent),

    // --- RFD ---
    RFDPickedFile(PathBuf),
    RFDPickError(String),
    RFDCancelPick,
    RFDSavedFile(PathBuf),
}

#[derive(Debug, Clone)]
pub enum ToBackend {
    Logout,
    // -------- REQWEST MESSAGES
    FetchLatestVersion,
    // -------- DATABASE MESSAGES
    QueryFirstLogin,
    PrepareClient,
    FetchRelayResponsesChatMsg(ChatMessage),
    FetchRelayResponsesUserProfile,
    FetchRelayResponsesContactList,
    FetchMessages(DbContact),

    FetchContacts,
    AddContact(DbContact),
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    ImportContacts((Vec<DbContact>, bool)),

    FetchRelays,
    GetUserProfileMeta,
    UpdateUserProfileMeta(nostr::Metadata),
    FetchAllMessageEvents,
    ExportMessages(Vec<DbEvent>),
    ExportContacts,
    FetchChatInfo(DbContact),
    // GetDbEventWithHash(nostr::EventId),
    FetchContactWithMetadata(XOnlyPublicKey),
    // -------- NOSTR CLIENT MESSAGES
    RefreshContactsProfile,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    SendDM((DbContact, String)),
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
    LoginWithSK(Keys),
    CreateAccount(BasicProfile),
}

pub async fn process_message(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    nostr: &RelayPool,
    backend: &mut BackendState,
    task_tx: &tokio::sync::mpsc::Sender<Result<TaskOutput, Error>>,
    message: ToBackend,
) -> Result<(), Error> {
    tracing::debug!("Processing message: {:?}", message);
    let pool = &backend.db_client.pool;
    let cache_pool = &backend.db_client.cache_pool;
    match message {
        // ---- CONFIG ----
        ToBackend::LoginWithSK(_) => {
            unreachable!("Login with sk client should be sent only once")
        }
        ToBackend::Logout => {
            unreachable!("Logout should be processed outside here")
        }
        ToBackend::CreateAccount(profile) => {
            let profile_meta: nostr::Metadata = profile.into();
            let ns_event = build_profile_event(pool, &keys, &profile_meta).await?;
            let pending_event = insert_pending(ns_event, pool, nostr).await?;
            let _ = output
                .send(BackendEvent::PendingMetadata(pending_event))
                .await;
        }
        // --- RFD ---
        ToBackend::ExportMessages(messages) => {
            let result = export_messages(&messages).await;
            match result {
                Ok(event) => {
                    let _ = output.send(event).await;
                }
                Err(e) => {
                    tracing::error!("Failed to export messages: {}", e);
                    let _ = output.send(BackendEvent::RFDPickError(e.to_string())).await;
                }
            }
        }
        ToBackend::ExportContacts => {
            let result = export_contacts(keys, pool).await;
            match result {
                Ok(event) => {
                    let _ = output.send(event).await;
                }
                Err(e) => {
                    tracing::error!("Failed to export contacts: {}", e);
                    let _ = output.send(BackendEvent::RFDPickError(e.to_string())).await;
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
                    let _ = output
                        .send(BackendEvent::RFDPickedFile(handle.path().to_owned()))
                        .await;
                }
                None => {
                    let _ = output.send(BackendEvent::RFDCancelPick).await;
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
            let _ = output.send(BackendEvent::FetchingLatestVersion).await;
        }
        ToBackend::DownloadImage {
            image_url,
            public_key,
            kind,
        } => {
            let task_tx_1 = task_tx.clone();
            tokio::spawn(async move {
                let result = download_image(&image_url, &public_key, kind)
                    .await
                    .map(TaskOutput::ImageDownloaded)
                    .map_err(|e| e.into());
                if let Err(e) = task_tx_1.send(result).await {
                    tracing::error!("Error sending image downloaded event: {}", e);
                }
            });
            let _ = output
                .send(BackendEvent::DownloadingImage {
                    kind: kind.clone(),
                    public_key: public_key.clone(),
                })
                .await;
        }
        // -----------
        ToBackend::UpdateUserProfileMeta(profile_meta) => {
            let ns_event = build_profile_event(pool, keys, &profile_meta).await?;
            let pending_event = insert_pending(ns_event, pool, nostr).await?;
            let _ = output
                .send(BackendEvent::PendingMetadata(pending_event))
                .await;
        }
        ToBackend::QueryFirstLogin => {
            if UserConfig::query_has_logged_in(pool).await? {
                prepare_client(pool, keys, nostr).await?;
                let _ = output.send(BackendEvent::FinishedPreparing).await;
            } else {
                let _ = output.send(BackendEvent::FirstLoginSuccess).await;
            }
        }
        ToBackend::PrepareClient => {
            prepare_client(pool, keys, nostr).await?;
            let _ = output.send(BackendEvent::FinishedPreparing).await;
        }
        ToBackend::RemoveFileFromCache((cache, kind)) => {
            // TODO: talvez um spawn task?
            ProfileCache::remove_file(cache_pool, &cache, kind).await?;
            let _ = output
                .send(BackendEvent::CacheFileRemoved((cache, kind)))
                .await;
        }
        ToBackend::FetchMoreMessages((db_contact, first_message)) => {
            let msgs = DbMessage::fetch_more(pool, db_contact.pubkey(), first_message).await?;
            match msgs.is_empty() {
                // update nostr subscriber
                true => (),
                false => {
                    let mut chat_messages = vec![];
                    tracing::debug!("Decrypting messages");
                    for db_message in &msgs {
                        let chat_message =
                            ChatMessage::from_db_message(&keys, &db_message, &db_contact)?;
                        chat_messages.push(chat_message);
                    }
                    let _ = output
                        .send(BackendEvent::GotChatMessages((db_contact, chat_messages)))
                        .await;
                }
            }
        }
        ToBackend::FetchContactWithMetadata(pubkey) => {
            let req = DbContact::fetch_one(pool, cache_pool, &pubkey).await?;
            let _ = output
                .send(BackendEvent::GotSingleContact((pubkey, req)))
                .await;
        }
        ToBackend::FetchRelayResponsesUserProfile => {
            if let Some(profile_event) =
                DbEvent::fetch_last_kind_pubkey(pool, nostr::Kind::Metadata, &keys.public_key())
                    .await?
            {
                let all_relays = DbRelay::fetch(pool).await?;
                let responses =
                    DbRelayResponse::fetch_by_event(pool, profile_event.event_id()?).await?;
                let _ = output
                    .send(BackendEvent::GotRelayResponsesUserProfile {
                        responses,
                        all_relays,
                    })
                    .await;
            }
        }
        ToBackend::FetchRelayResponsesContactList => {
            if let Some(profile_event) =
                DbEvent::fetch_last_kind_pubkey(pool, nostr::Kind::ContactList, &keys.public_key())
                    .await?
            {
                let all_relays = DbRelay::fetch(pool).await?;
                let responses =
                    DbRelayResponse::fetch_by_event(pool, profile_event.event_id()?).await?;
                let _ = output
                    .send(BackendEvent::GotRelayResponsesContactList {
                        responses,
                        all_relays,
                    })
                    .await;
            }
        }
        ToBackend::FetchRelayResponsesChatMsg(chat_message) => {
            if let Some(db_message) = DbMessage::fetch_one(pool, chat_message.msg_id).await? {
                let all_relays = DbRelay::fetch(pool).await?;
                let responses =
                    DbRelayResponse::fetch_by_event(pool, db_message.event_id()?).await?;
                let _ = output
                    .send(BackendEvent::GotRelayResponses {
                        responses,
                        all_relays,
                        chat_message,
                    })
                    .await;
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
            let _ = output
                .send(BackendEvent::GotChatInfo((db_contact, chat_info)))
                .await;
        }
        ToBackend::FetchAllMessageEvents => {
            let messages = DbEvent::fetch_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
            let _ = output.send(BackendEvent::GotAllMessages(messages)).await;
        }
        ToBackend::GetUserProfileMeta => {
            let cache = ProfileCache::fetch_by_public_key(cache_pool, &keys.public_key()).await?;
            let _ = output.send(BackendEvent::GotUserProfileCache(cache)).await;
        }
        ToBackend::ImportContacts((db_contacts, is_replace)) => {
            for db_contact in &db_contacts {
                // check if contact already exists
                // if exists and replace just upsert
                // else none
                todo!();
            }
            send_contact_list(pool, keys, nostr).await?;
            let _ = output
                .send(BackendEvent::FileContactsImported(db_contacts))
                .await;
        }
        ToBackend::AddContact(db_contact) => {
            // Check if the contact is the same as the user
            if &keys.public_key() == db_contact.pubkey() {
                return Err(Error::SameContactInsert);
            }
            // add or update basic
            DbContact::upsert_contact(pool, &db_contact).await?;
            send_contact_list(pool, keys, nostr).await?;
            let _ = output.send(BackendEvent::ContactCreated(db_contact)).await;
        }
        ToBackend::UpdateContact(db_contact) => {
            if &keys.public_key() == db_contact.pubkey() {
                return Err(Error::SameContactUpdate);
            }
            DbContact::update(pool, &db_contact).await?;
            send_contact_list(pool, keys, nostr).await?;
            let _ = output.send(BackendEvent::ContactUpdated(db_contact)).await;
        }
        ToBackend::DeleteContact(db_contact) => {
            DbContact::delete(pool, &db_contact).await?;
            send_contact_list(pool, keys, nostr).await?;
            let _ = output.send(BackendEvent::ContactDeleted(db_contact)).await;
        }
        ToBackend::FetchContacts => {
            let contacts = DbContact::fetch(pool, cache_pool).await?;
            let _ = output.send(BackendEvent::GotContacts(contacts)).await;
        }
        ToBackend::FetchRelays => {
            let relays = DbRelay::fetch(pool).await?;
            let _ = output.send(BackendEvent::GotRelays(relays)).await;
        }
        ToBackend::FetchMessages(db_contact) => {
            let mut db_messages = DbMessage::fetch_chat(pool, db_contact.pubkey()).await?;
            let mut chat_messages = vec![];

            // Maybe the message is only seen when scrolling?
            tracing::debug!("Updating unseen messages to marked as seen");
            for db_message in db_messages.iter_mut().filter(|m| m.is_unseen()) {
                DbMessage::message_seen(pool, db_message).await?;
            }

            // Maybe a spawned task?
            tracing::debug!("Decrypting messages");
            for db_message in &db_messages {
                let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
                chat_messages.push(chat_message);
            }

            let _ = output
                .send(BackendEvent::GotChatMessages((db_contact, chat_messages)))
                .await;
        }
        // ToBackend::GetDbEventWithHash(event_hash) => {
        //     let db_event_opt = DbEvent::fetch_one(pool, &event_hash).await?;
        //     let _ = output.send(BackendEvent::GotDbEvent(db_event_opt)).await;
        // }
        ToBackend::GetRelayStatusList => {
            let list = nostr.relay_status_list().await?;
            let _ = output.send(BackendEvent::GotRelayStatusList(list)).await;
        }
        ToBackend::RefreshContactsProfile => {
            // subscribe_eose contact_list_metadata
            todo!();
        }
        ToBackend::CreateChannel => {
            todo!()
        }
        ToBackend::AddRelay(db_relay) => {
            let opts = ns_client::RelayOptions::new(db_relay.read, db_relay.write);
            nostr.add_relay_with_opts(db_relay.url.as_str(), opts)?;
            DbRelay::insert(pool, &db_relay).await?;
            let _ = output.send(BackendEvent::RelayCreated(db_relay)).await;
        }
        ToBackend::DeleteRelay(db_relay) => {
            nostr.remove_relay(db_relay.url.as_str())?;
            DbRelay::delete(pool, &db_relay).await?;
            let _ = output.send(BackendEvent::RelayDeleted(db_relay)).await;
        }
        ToBackend::ToggleRelayRead((mut db_relay, read)) => {
            nostr.toggle_read_for(&db_relay.url, read)?;
            db_relay.read = read;
            DbRelay::update(&pool, &db_relay).await?;
            let _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
        }
        ToBackend::ToggleRelayWrite((mut db_relay, write)) => {
            nostr.toggle_write_for(&db_relay.url, write)?;
            db_relay.write = write;
            DbRelay::update(&pool, &db_relay).await?;
            let _ = output.send(BackendEvent::RelayUpdated(db_relay)).await;
        }
        ToBackend::SendDM((db_contact, content)) => {
            // create a pending event and await confirmation of relays
            let ns_event = build_dm(pool, keys, &db_contact, &content).await?;
            let pending_event = insert_pending(ns_event, pool, nostr).await?;
            let pending_msg = DbMessage::new(&pending_event, db_contact.pubkey())?;
            let row_id = DbMessage::insert_message(pool, &pending_msg).await?;
            let pending_msg = pending_msg.with_id(row_id);
            let chat_message =
                ChatMessage::from_db_message_content(keys, &pending_msg, &db_contact, &content)?;
            let _ = output
                .send(BackendEvent::PendingDM((db_contact, chat_message)))
                .await;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    ContactList,
    ContactListMetadata,
    UserMetadata,
    Messages,
    Channel,
    ChannelMetadata,
}
impl SubscriptionType {
    pub const ALL: [SubscriptionType; 6] = [
        SubscriptionType::ContactList,
        SubscriptionType::ContactListMetadata,
        SubscriptionType::UserMetadata,
        SubscriptionType::Messages,
        SubscriptionType::Channel,
        SubscriptionType::ChannelMetadata,
    ];
}
impl std::fmt::Display for SubscriptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionType::ContactList => write!(f, "ContactList"),
            SubscriptionType::ContactListMetadata => write!(f, "ContactListMetadata"),
            SubscriptionType::UserMetadata => write!(f, "UserMetadata"),
            SubscriptionType::Messages => write!(f, "Messages"),
            SubscriptionType::Channel => write!(f, "Channel"),
            SubscriptionType::ChannelMetadata => write!(f, "ChannelMetadata"),
        }
    }
}
pub fn contact_list_metadata(contact_list: &[DbContact]) -> Filter {
    let all_pubkeys = contact_list
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();
    Filter::new().authors(all_pubkeys).kind(Kind::Metadata)
    // .since(Timestamp::from(last_timestamp_secs))
}
pub fn user_metadata(pubkey: XOnlyPublicKey) -> Filter {
    Filter::new()
        .author(pubkey.to_string())
        .kind(Kind::Metadata)
}

fn contact_list_filter(public_key: XOnlyPublicKey, last_timestamp_secs: u64) -> Filter {
    let user_contact_list = Filter::new()
        .author(public_key.to_string())
        .kind(Kind::ContactList)
        .since(Timestamp::from(last_timestamp_secs));
    user_contact_list
}

fn messages_filter(public_key: XOnlyPublicKey, last_timestamp_secs: u64) -> Vec<Filter> {
    let sent_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .author(public_key.to_string())
        .since(Timestamp::from(last_timestamp_secs));
    let recv_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .pubkey(public_key)
        .since(Timestamp::from(last_timestamp_secs));
    vec![sent_msgs, recv_msgs]
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

    let rows_changed = ProfileCache::insert_with_event(cache_pool, db_event).await?;

    if rows_changed == 0 {
        tracing::debug!("Cache already up to date");
    }

    let _ = output
        .send(BackendEvent::UpdatedMetadata(db_event.pubkey))
        .await;

    Ok(())
}

async fn handle_channel_creation(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<(), Error> {
    tracing::debug!("handle_channel_creation");
    let channel_cache = ChannelCache::insert(cache_pool, db_event).await?;
    let _ = output
        .send(BackendEvent::ChannelCreated(channel_cache))
        .await;
    Ok(())
}

async fn handle_channel_update(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    cache_pool: &SqlitePool,
    db_event: &DbEvent,
) -> Result<(), Error> {
    tracing::debug!("handle_channel_update");
    ChannelCache::update(cache_pool, db_event).await?;
    let channel_cache = ChannelCache::update(cache_pool, db_event).await?;
    let _ = output
        .send(BackendEvent::ChannelUpdated(channel_cache))
        .await;
    Ok(())
}

pub async fn handle_recommend_relay(db_event: DbEvent) -> Result<(), Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&db_event);
    Ok(())
}

pub async fn create_channel(_client: &RelayPool) -> Result<BackendEvent, Error> {
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
    client: &RelayPool,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
) -> Result<(), Error> {
    tracing::info!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        let opts = ns_client::RelayOptions::new(r.read, r.write);
        match client.add_relay_with_opts(&r.url.to_string(), opts) {
            Ok(_) => tracing::debug!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    let last_timestamp_secs: u64 = last_event
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.local_creation - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0);
    tracing::info!("last event timestamp: {}", last_timestamp_secs);

    let contact_list_sub_id = SubscriptionId::new(SubscriptionType::ContactList.to_string());
    client.subscribe_eose(
        &contact_list_sub_id,
        vec![contact_list_filter(keys.public_key(), last_timestamp_secs)],
    )?;
    let user_metadata_id = SubscriptionId::new(SubscriptionType::UserMetadata.to_string());
    client.subscribe_eose(&user_metadata_id, vec![user_metadata(keys.public_key())])?;
    let messages_sub_id = SubscriptionId::new(SubscriptionType::Messages.to_string());
    client.subscribe_id(
        &messages_sub_id,
        messages_filter(keys.public_key(), last_timestamp_secs),
    )?;
    // client.subscribe_eose(id, channel_filter)?;

    Ok(())
}
fn channel_search_filter() -> Vec<Filter> {
    tracing::trace!("channel_search_filter");

    let channel_filter = Filter::new()
        .kinds(vec![Kind::ChannelCreation, Kind::ChannelMetadata])
        .limit(CHANNEL_SEARCH_LIMIT);

    vec![channel_filter]
}

async fn prepare_client(pool: &SqlitePool, keys: &Keys, nostr: &RelayPool) -> Result<(), Error> {
    let relays = DbRelay::fetch(pool).await?;
    let last_event = DbEvent::fetch_last_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
    UserConfig::store_first_login(pool).await?;
    add_relays_and_connect(nostr, &keys, &relays, last_event)?;
    Ok(())
}

async fn send_contact_list(pool: &SqlitePool, keys: &Keys, nostr: &RelayPool) -> Result<(), Error> {
    let list = DbContact::fetch_basic(pool).await?;
    let ns_event = build_contact_list_event(pool, keys, &list).await?;
    let _pending_event = insert_pending(ns_event, pool, nostr).await?;
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
async fn export_messages(messages: &[DbEvent]) -> Result<BackendEvent, Error> {
    let ns_events: Result<Vec<_>, _> = messages.iter().map(|m| m.to_ns_event()).collect();
    let ns_events = ns_events?; // Unwrap the Result, propagating any errors.
    save_file(&ns_events, "json").await
}
async fn export_contacts(keys: &Keys, pool: &SqlitePool) -> Result<BackendEvent, Error> {
    let list = DbContact::fetch_basic(pool).await?;
    let event = build_contact_list_event(pool, keys, &list).await?;
    save_file(&event, "json").await
}

const CHANNEL_SEARCH_LIMIT: usize = 100;
const BACKEND_CHANNEL_SIZE: usize = 100;
