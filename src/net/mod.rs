use crate::db::{
    fetch_user_meta, query_has_logged_in, store_first_login, update_user_meta, Database, DbContact,
    DbEvent,
};
use crate::error::Error;
use crate::net::events::backend::{self, backend_processing};
use crate::net::events::frontend::Event;
use crate::net::events::nostr::NostrSdkWrapper;
use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use futures::{Future, StreamExt};
use iced::futures::stream::Fuse;
use iced::subscription;
use iced::Subscription;
use nostr_sdk::{Keys, RelayPoolNotification};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::env;
use std::pin::Pin;
use std::time::Duration;

mod back_channel;
pub(crate) mod events;
mod messages;

pub(crate) use self::back_channel::BackEndConnection;
use self::events::backend::{
    add_to_unseen_count, delete_contact, fetch_and_decrypt_chat, fetch_contacts, fetch_relays,
    fetch_relays_responses, insert_contact, prepare_client, update_contact, BackEndInput,
};
use self::events::nostr::{build_dm, NostrInput, NostrOutput};
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
        let (sender, receiver) = mpsc::channel(100);
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

    pub async fn process_in(&mut self, backend_input: BackEndInput) -> Event {
        if let Some(db_client) = &mut self.db_client {
            backend_processing(&db_client.pool, &self.keys, backend_input, &mut self.sender).await
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
        client_sender: &mut mpsc::Sender<NostrInput>,
        message: Message,
    ) -> Event {
        if let (Some(db_client), Some(req_client)) = (&mut self.db_client, &mut self.req_client) {
            let pool = &db_client.pool;
            match message {
                // ---- REQWEST ----
                Message::FetchLatestVersion => {
                    tracing::info!("Fetching latest version");
                    let req_client_clone = req_client.clone();
                    let mut sender_clone = self.sender.clone();
                    tokio::spawn(async move {
                        match fetch_latest_version(req_client_clone).await {
                            Ok(version) => {
                                if let Err(e) =
                                    sender_clone.try_send(BackEndInput::LatestVersion(version))
                                {
                                    tracing::error!("Error sending latest version: {}", e);
                                }
                            }
                            Err(e) => tracing::error!("{}", e),
                        }
                    });
                    Event::FetchingLatestVersion
                }
                // ---- CONFIG ----
                Message::Logout => {
                    panic!("Logout should be processed outside here")
                }
                Message::CreateAccount(profile) => {
                    tracing::info!("Creating account");
                    let profile_meta = profile.into();
                    match update_user_meta(pool, &profile_meta).await {
                        Ok(_) => {
                            to_nostr_channel(client_sender, NostrInput::SendProfile(profile_meta));
                            Event::ProfileCreated
                        }
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::StoreFirstLogin => match store_first_login(pool).await {
                    Ok(_) => Event::FirstLoginStored,
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::QueryFirstLogin => match handle_first_login(pool, client_sender).await {
                    Ok(event) => event,
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::PrepareClient => match prepare_client(pool).await {
                    Ok(input) => to_nostr_channel(client_sender, input),
                    Err(e) => Event::Error(e.to_string()),
                },
                // -------- DATABASE MESSAGES -------
                Message::GetUserProfileMeta => {
                    tracing::info!("Fetching user profile meta");
                    match fetch_user_meta(pool).await {
                        Ok(Some(meta)) => Event::GotUserProfileMeta(meta),
                        Ok(None) => Event::None,
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::UpdateUserProfileMeta(profile_meta) => {
                    tracing::info!("Updating user profile meta");
                    match update_user_meta(pool, &profile_meta).await {
                        Ok(_) => {
                            to_nostr_channel(
                                client_sender,
                                NostrInput::SendProfile(profile_meta.clone()),
                            );
                            Event::UpdatedUserProfileMeta(profile_meta)
                        }
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::RequestEvents => match DbEvent::fetch_last(pool).await {
                    Ok(last_event) => {
                        to_nostr_channel(client_sender, NostrInput::RequestEvents(last_event))
                    }
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::RequestEventsOf(db_relay) => {
                    to_nostr_channel(client_sender, NostrInput::RequestEventsOf(db_relay))
                }
                Message::FetchRelayResponses(event_id) => {
                    process_async_with_event(fetch_relays_responses(pool, event_id)).await
                }
                Message::AddToUnseenCount(db_contact) => {
                    process_async_with_event(add_to_unseen_count(pool, db_contact)).await
                }
                Message::ImportContacts(db_contacts) => {
                    for db_contact in &db_contacts {
                        if let Err(e) = insert_contact(&self.keys, pool, db_contact).await {
                            tracing::error!("{}", e);
                        }
                    }
                    Event::FileContactsImported(db_contacts)
                }
                Message::AddContact(db_contact) => {
                    process_async_with_event(insert_contact(&self.keys, pool, &db_contact)).await
                }
                Message::UpdateContact(db_contact) => {
                    process_async_with_event(update_contact(&self.keys, pool, &db_contact)).await
                }
                Message::DeleteContact(contact) => {
                    process_async_with_event(delete_contact(pool, &contact)).await
                }
                Message::FetchContacts => process_async_with_event(fetch_contacts(pool)).await,
                Message::FetchRelays => process_async_with_event(fetch_relays(pool)).await,
                Message::FetchMessages(contact) => {
                    process_async_with_event(fetch_and_decrypt_chat(&self.keys, pool, contact))
                        .await
                }
                // --------- NOSTR MESSAGES ------------
                Message::GetContactProfile(db_contact) => {
                    to_nostr_channel(client_sender, NostrInput::GetContactProfile(db_contact))
                }
                Message::SendDMToRelays(chat_msg) => {
                    match DbEvent::fetch_one(pool, &chat_msg.event_hash).await {
                        Ok(Some(db_event)) => {
                            to_nostr_channel(client_sender, NostrInput::SendDMToRelays(db_event))
                        }
                        Ok(None) => Event::None,
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
                Message::FetchRelayServer(url) => {
                    to_nostr_channel(client_sender, NostrInput::FetchRelayServer(url))
                }
                Message::FetchRelayServers => {
                    to_nostr_channel(client_sender, NostrInput::FetchRelayServers)
                }
                Message::CreateChannel => {
                    to_nostr_channel(client_sender, NostrInput::CreateChannel)
                }
                Message::ConnectToRelay(db_relay) => match DbEvent::fetch_last(pool).await {
                    Ok(last_event) => to_nostr_channel(
                        client_sender,
                        NostrInput::ConnectToRelay((db_relay, last_event)),
                    ),
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::AddRelay(db_relay) => {
                    to_nostr_channel(client_sender, NostrInput::AddRelay(db_relay))
                }
                Message::DeleteRelay(db_relay) => {
                    to_nostr_channel(client_sender, NostrInput::DeleteRelay(db_relay))
                }
                Message::ToggleRelayRead((db_relay, read)) => {
                    to_nostr_channel(client_sender, NostrInput::ToggleRelayRead((db_relay, read)))
                }
                Message::ToggleRelayWrite((db_relay, write)) => to_nostr_channel(
                    client_sender,
                    NostrInput::ToggleRelayWrite((db_relay, write)),
                ),
                Message::SendContactListToRelay(db_relay) => match DbContact::fetch(pool).await {
                    Ok(list) => to_nostr_channel(
                        client_sender,
                        NostrInput::SendContactListToRelay((db_relay, list)),
                    ),
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::BuildDM((db_contact, content)) => {
                    // to_nostr_channel(client_sender, NostrInput::SendDMTo((contact, msg)))
                    match build_dm(&self.keys, &db_contact, &content) {
                        Ok(input) => to_backend_channel(&mut self.sender, input),
                        Err(e) => Event::Error(e.to_string()),
                    }
                }
            }
        } else {
            Event::None
        }
    }

    pub async fn process_notification(&mut self, notification: RelayPoolNotification) -> Event {
        // Cria uma tarefa separada para processar a notificação
        let backend_input = match notification {
            RelayPoolNotification::Event(relay_url, event) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
                backend::BackEndInput::StoreEvent((relay_url, event))
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
        let (in_sender, in_receiver) = mpsc::channel(100);
        let (out_sender, out_receiver) = mpsc::channel(100);
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
                    let backend = backend.with_req(req_client);

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
                            backend.process_in(backend_input).await
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

// ---------------------------------

async fn process_async_fn<T, E>(
    operation: impl Future<Output = Result<T, E>>,
    success_event_fn: impl Fn(T) -> Event,
) -> Event
where
    E: std::error::Error,
{
    match operation.await {
        Ok(result) => success_event_fn(result),
        Err(e) => Event::Error(e.to_string()),
    }
}

async fn process_async_with_event<E>(operation: impl Future<Output = Result<Event, E>>) -> Event
where
    E: std::error::Error,
{
    match operation.await {
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

fn to_nostr_channel(client_sender: &mut mpsc::Sender<NostrInput>, input: NostrInput) -> Event {
    try_send_to_channel(
        client_sender,
        input,
        Event::NostrLoading,
        "Error sending to Nostr channel",
    )
}

fn to_backend_channel(
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

async fn handle_first_login(
    pool: &SqlitePool,
    client_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    if query_has_logged_in(pool).await? {
        let input = prepare_client(pool).await?;
        to_nostr_channel(client_sender, input);
        Ok(Event::None)
    } else {
        Ok(Event::FirstLogin)
    }
}

#[derive(Deserialize, Debug)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
}

async fn fetch_latest_version(client: reqwest::Client) -> Result<String, Error> {
    let token = env::var("GITHUB_TOKEN").map_err(|_| Error::GitHubTokenNotFound)?;
    let url = "https://api.github.com/repos/luizvidoto/nostrtalk/releases";
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", token))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    response.error_for_status_ref()?;

    let releases: Vec<GitHubRelease> = response.json().await?;

    // Ordena as releases por data de criação
    let mut sorted_releases: Vec<&GitHubRelease> = releases.iter().collect();
    sorted_releases.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let first_release = sorted_releases
        .get(0)
        .ok_or(Error::RequestReleaseNotFound)?;
    Ok(first_release.tag_name.clone())
}
