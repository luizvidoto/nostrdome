use crate::db::{Database, DbContact, DbEvent, DbRelay, DbRelayResponse, UserConfig};
use crate::error::Error;
use crate::net::events::backend::{self, backend_processing};
use crate::net::events::frontend::Event;
use crate::net::events::nostr::NostrSdkWrapper;
use crate::net::reqwest_client::fetch_latest_version;
use crate::ntp::ntp_request;
use futures::channel::mpsc;
use futures::{Future, StreamExt};
use iced::futures::stream::Fuse;
use iced::subscription;
use iced::Subscription;
use nostr_sdk::{Keys, RelayPoolNotification};
use sqlx::SqlitePool;
use std::pin::Pin;

mod back_channel;
pub(crate) mod events;
mod messages;
mod operations;
mod reqwest_client;

pub(crate) use reqwest_client::{get_png_image_path, ImageKind, ImageSize};

pub(crate) use self::back_channel::BackEndConnection;
use self::events::backend::BackEndInput;
use self::events::nostr::{prepare_client, NostrInput, NostrOutput};
use self::operations::builder::{build_contact_list_event, build_dm};
use self::operations::contact::{
    add_new_contact, delete_contact, fetch_and_decrypt_chat, import_contacts, update_contact,
};
use self::operations::metadata::update_user_metadata;
pub(crate) use messages::Message;

pub struct BackendState {
    keys: Keys,
    db_client: Option<Database>,
    sender: mpsc::Sender<BackEndInput>,
    receiver: mpsc::Receiver<BackEndInput>,
    req_client: Option<reqwest::Client>,
}
impl BackendState {
    pub fn new(keys: &Keys) -> Self {
        let (sender, receiver) = mpsc::channel(1024);

        Self {
            sender,
            receiver,
            keys: keys.to_owned(),
            db_client: None,
            req_client: None,
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

    pub async fn process_in(
        &mut self,
        backend_input: BackEndInput,
        nostr_sender: &mut mpsc::Sender<NostrInput>,
    ) -> Event {
        if let Some(db_client) = &mut self.db_client {
            backend_processing(
                &db_client.pool,
                &self.keys,
                backend_input,
                &mut self.sender,
                nostr_sender,
            )
            .await
        } else {
            Event::None
        }
    }
    pub async fn logout(self) -> Result<(), Error> {
        tracing::info!("Database Logging out");
        Ok(())
    }
    pub async fn process_message(
        &mut self,
        nostr_sender: &mut mpsc::Sender<NostrInput>,
        message: Message,
    ) -> Event {
        if let (Some(db_client), Some(req_client)) = (&mut self.db_client, &mut self.req_client) {
            let pool = &db_client.pool;
            match message {
                // ---- REQWEST ----
                Message::FetchLatestVersion => {
                    handle_fetch_last_version(req_client, &mut self.sender)
                }
                // ---- CONFIG ----
                Message::Logout => {
                    panic!("Logout should be processed outside here")
                }
                Message::CreateAccount(profile) => {
                    let profile_meta = profile.into();
                    update_user_metadata(pool, &self.keys, &mut self.sender, profile_meta).await
                }
                Message::UpdateUserProfileMeta(profile_meta) => {
                    update_user_metadata(pool, &self.keys, &mut self.sender, profile_meta).await
                }
                Message::StoreFirstLogin => match UserConfig::store_first_login(pool).await {
                    Ok(_) => Event::FirstLoginStored,
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::QueryFirstLogin => match handle_first_login(pool, nostr_sender).await {
                    Ok(event) => event,
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::PrepareClient => match prepare_client(pool).await {
                    Ok(input) => to_nostr_channel(nostr_sender, input),
                    Err(e) => Event::Error(e.to_string()),
                },
                // -------- DATABASE MESSAGES -------
                Message::GetUserProfileMeta => {
                    tracing::info!("Fetching user profile meta");
                    match UserConfig::fetch(pool).await {
                        Ok(user) => Event::GotUserProfileMeta(user.profile_meta),
                        Err(e) => Event::Error(e.to_string()),
                    }
                }

                Message::FetchRelayResponses(event_id) => {
                    match DbRelayResponse::fetch_by_event(pool, event_id).await {
                        Ok(responses) => Event::GotRelayResponses(responses),
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::AddToUnseenCount(db_contact) => {
                    match DbContact::add_to_unseen_count(pool, db_contact).await {
                        Ok(db_contact) => Event::ContactUpdated(db_contact),
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::ImportContacts((db_contacts, is_replace)) => {
                    process_async_with_event(import_contacts(
                        &self.keys,
                        pool,
                        db_contacts,
                        is_replace,
                    ))
                    .await
                }
                Message::AddContact(db_contact) => {
                    process_async_with_event(add_new_contact(&self.keys, pool, &db_contact)).await
                }
                Message::UpdateContact(db_contact) => {
                    process_async_with_event(update_contact(&self.keys, pool, &db_contact)).await
                }
                Message::DeleteContact(contact) => {
                    process_async_with_event(delete_contact(pool, &contact)).await
                }
                Message::FetchContacts => match DbContact::fetch(pool).await {
                    Ok(contacts) => Event::GotContacts(contacts),
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::FetchRelays => match DbRelay::fetch(pool).await {
                    Ok(relays) => Event::GotRelays(relays),
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::FetchMessages(contact) => {
                    process_async_with_event(fetch_and_decrypt_chat(&self.keys, pool, contact))
                        .await
                }
                // --------- NOSTR MESSAGES ------------
                Message::RequestEventsOf(db_relay) => match DbContact::fetch(pool).await {
                    Ok(contact_list) => to_nostr_channel(
                        nostr_sender,
                        NostrInput::RequestEventsOf((db_relay, contact_list)),
                    ),
                    Err(e) => Event::Error(e.to_string()),
                },

                Message::RefreshContactsProfile => match DbContact::fetch(pool).await {
                    Ok(db_contacts) => {
                        to_nostr_channel(
                            nostr_sender,
                            NostrInput::GetContactListProfiles(db_contacts),
                        );
                        Event::None
                    }
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::FetchRelayServer(url) => {
                    to_nostr_channel(nostr_sender, NostrInput::FetchRelayServer(url))
                }
                Message::FetchRelayServers => {
                    to_nostr_channel(nostr_sender, NostrInput::FetchRelayServers)
                }
                Message::CreateChannel => to_nostr_channel(nostr_sender, NostrInput::CreateChannel),
                Message::ConnectToRelay(db_relay) => match DbEvent::fetch_last(pool).await {
                    Ok(last_event) => to_nostr_channel(
                        nostr_sender,
                        NostrInput::ConnectToRelay((db_relay, last_event)),
                    ),
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::AddRelay(db_relay) => {
                    to_nostr_channel(nostr_sender, NostrInput::AddRelay(db_relay))
                }
                Message::DeleteRelay(db_relay) => {
                    to_nostr_channel(nostr_sender, NostrInput::DeleteRelay(db_relay))
                }
                Message::ToggleRelayRead((db_relay, read)) => {
                    to_nostr_channel(nostr_sender, NostrInput::ToggleRelayRead((db_relay, read)))
                }
                Message::ToggleRelayWrite((db_relay, write)) => to_nostr_channel(
                    nostr_sender,
                    NostrInput::ToggleRelayWrite((db_relay, write)),
                ),

                // -- TO BACKEND CHANNEL --
                Message::SendDM((db_contact, content)) => {
                    // to backend channel, create a pending event and await confirmation of relays
                    match build_dm(pool, &self.keys, &db_contact, &content).await {
                        Ok(ns_event) => to_backend_channel(
                            &mut self.sender,
                            BackEndInput::StorePendingMessage {
                                ns_event,
                                content,
                                db_contact,
                            },
                        ),
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::SendContactListToRelay(_db_relay) => match DbContact::fetch(pool).await {
                    Ok(list) => match build_contact_list_event(pool, &self.keys, &list).await {
                        Ok(ns_event) => to_backend_channel(
                            &mut self.sender,
                            BackEndInput::StorePendingContactList((ns_event, list.to_owned())),
                        ),
                        Err(e) => Event::Error(e.to_string()),
                    },
                    Err(e) => Event::Error(e.to_string()),
                },
            }
        } else {
            Event::None
        }
    }

    pub async fn process_notification(&mut self, notification: RelayPoolNotification) -> Event {
        // Cria uma tarefa separada para processar a notificação
        let backend_input = match notification {
            RelayPoolNotification::Event(relay_url, event) => {
                backend::BackEndInput::StoreConfirmedEvent((relay_url, event))
            }
            RelayPoolNotification::Message(relay_url, msg) => {
                backend::BackEndInput::StoreRelayMessage((relay_url, msg))
            }
            RelayPoolNotification::Shutdown => backend::BackEndInput::Shutdown,
        };

        if let Err(e) = self.sender.try_send(backend_input) {
            tracing::error!("{}", e);
        }

        Event::None
    }
}

pub struct NostrState {
    keys: Keys,
    in_receiver: mpsc::Receiver<NostrInput>,
    in_sender: mpsc::Sender<NostrInput>,
    out_receiver: mpsc::Receiver<NostrOutput>,
    out_sender: mpsc::Sender<NostrOutput>,
    client: NostrSdkWrapper,
    notifications_stream: Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
}
impl NostrState {
    pub async fn new(keys: &Keys) -> Self {
        let (in_sender, in_receiver) = mpsc::channel(1024);
        let (out_sender, out_receiver) = mpsc::channel(1024);
        let (client, notifications_stream) = NostrSdkWrapper::new(&keys).await;
        Self {
            keys: keys.to_owned(),
            in_receiver,
            in_sender,
            out_receiver,
            out_sender,
            client,
            notifications_stream,
        }
    }
    pub async fn logout(self) -> Result<(), Error> {
        Ok(self.client.logout().await?)
    }
    pub async fn process_out(
        &mut self,
        nostr_output: NostrOutput,
        backend_channel: &mut mpsc::Sender<BackEndInput>,
    ) -> Event {
        match nostr_output {
            NostrOutput::Ok(event) => event,
            NostrOutput::Error(e) => Event::Error(e.to_string()),
            NostrOutput::ToBackend(input) => to_backend_channel(backend_channel, input),
        }
    }
    pub async fn process_in(&mut self, nostr_input: NostrInput) -> Event {
        self.client
            .process_in(&mut self.out_sender, &self.keys, nostr_input)
            .await;
        Event::None
    }
}

pub enum State {
    End,
    Start {
        keys: Keys,
        backend: BackendState,
    },
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        backend: BackendState,
        nostr: NostrState,
    },
    OnlyNostr {
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr: NostrState,
    },
}
impl State {
    pub fn start(keys: Keys) -> Self {
        let backend = BackendState::new(&keys);
        Self::Start { keys, backend }
    }
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<Event>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub =
        subscription::unfold(id, State::start(keys.to_owned()), |state| async move {
            match state {
                State::End => iced::futures::future::pending().await,
                State::OnlyNostr { receiver: _, nostr } => {
                    if let Err(e) = nostr.logout().await {
                        tracing::error!("{}", e);
                    }
                    (Event::LoggedOut, State::End)
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
                            return (Event::Disconnected, State::Start { keys, backend });
                        }
                    };

                    let backend = backend.with_db(db_client);
                    let mut backend = backend.with_req(req_client);

                    tracing::info!("Fetching NTP time to sync with system time");
                    ntp_request(&mut backend.sender);

                    (
                        Event::Connected(BackEndConnection::new(sender)),
                        State::Connected {
                            receiver,
                            backend,
                            nostr,
                        },
                    )
                }
                State::Connected {
                    mut receiver,
                    mut backend,
                    mut nostr,
                } => {
                    let event: Event = futures::select! {
                        message = receiver.select_next_some() => {
                            if let Message::Logout = message {
                                if let Err(e) = backend.logout().await {
                                    tracing::error!("{}", e);
                                }
                                return (Event::BackendClosed, State::OnlyNostr { receiver, nostr });
                            } else {
                                backend.process_message(&mut nostr.in_sender, message).await
                            }
                        }
                        nostr_input = nostr.in_receiver.select_next_some() => {
                            nostr.process_in(nostr_input).await
                        }
                        nostr_output = nostr.out_receiver.select_next_some() => {
                            nostr.process_out(nostr_output, &mut backend.sender).await
                        }
                        backend_input = backend.receiver.select_next_some() => {
                            backend.process_in(backend_input, &mut nostr.in_sender).await
                        }
                        notification = nostr.notifications_stream.select_next_some() => {
                            backend.process_notification(notification).await
                        },
                    };

                    (
                        event,
                        State::Connected {
                            receiver,
                            backend,
                            nostr,
                        },
                    )
                }
            }
        });

    vec![database_sub]
}

fn handle_fetch_last_version(
    req_client: &mut reqwest::Client,
    sender: &mut mpsc::Sender<BackEndInput>,
) -> Event {
    tracing::info!("Fetching latest version");
    let req_client_clone = req_client.clone();
    let mut sender_clone = sender.clone();
    tokio::spawn(async move {
        match fetch_latest_version(req_client_clone).await {
            Ok(version) => {
                if let Err(e) = sender_clone.try_send(BackEndInput::LatestVersion(version)) {
                    tracing::error!("Error sending latest version: {}", e);
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    });
    Event::FetchingLatestVersion
}

async fn handle_first_login(
    pool: &SqlitePool,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    if UserConfig::query_has_logged_in(pool).await? {
        let input = prepare_client(pool).await?;
        to_nostr_channel(nostr_sender, input);
        Ok(Event::None)
    } else {
        Ok(Event::FirstLogin)
    }
}

// ---------------------------------
// ---------------------------------
// ---------------------------------

async fn process_async_with_event<E>(op: impl Future<Output = Result<Event, E>>) -> Event
where
    E: std::error::Error,
{
    match op.await {
        Ok(result) => result,
        Err(e) => Event::Error(e.to_string()),
    }
}

pub fn try_send_to_channel<T>(
    channel: &mut mpsc::Sender<T>,
    input: T,
    ok_event: Event,
    error_msg: &str,
) -> Event {
    match channel.try_send(input) {
        Ok(_) => ok_event,
        Err(e) => Event::Error(format!("{}: {}", error_msg, e.to_string())),
    }
}

fn to_nostr_channel(nostr_sender: &mut mpsc::Sender<NostrInput>, input: NostrInput) -> Event {
    try_send_to_channel(
        nostr_sender,
        input,
        Event::NostrLoading,
        "Error sending to Nostr channel",
    )
}

pub fn to_backend_channel(
    backend_channel: &mut mpsc::Sender<BackEndInput>,
    input: BackEndInput,
) -> Event {
    try_send_to_channel(
        backend_channel,
        input,
        Event::None,
        "Error sending to Backend channel",
    )
}
