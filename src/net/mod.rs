use crate::db::{query_has_logged_in, store_first_login, Database, DbEvent};
use crate::error::Error;
use crate::net::events::backend::{self, backend_processing};
use crate::net::events::frontend::Event;
use crate::net::events::nostr::NostrSdkWrapper;
use crate::net::logic::{delete_contact, import_contacts, insert_contact, update_contact};
use futures::channel::mpsc;
use futures::{Future, StreamExt};
use iced::futures::stream::Fuse;
use iced::subscription;
use iced::Subscription;
use logic::*;
use nostr_sdk::{Keys, RelayPoolNotification};
use sqlx::SqlitePool;
use std::pin::Pin;

mod back_channel;
pub(crate) mod events;
mod logic;
mod messages;

pub(crate) use self::back_channel::BackEndConnection;
use self::events::backend::BackEndInput;
use self::events::nostr::{NostrInput, NostrOutput};
pub(crate) use messages::Message;

pub struct DatabaseState {
    keys: Keys,
    db_client: Option<Database>,
    sender: mpsc::Sender<BackEndInput>,
    receiver: mpsc::Receiver<BackEndInput>,
}
impl DatabaseState {
    pub fn new(keys: &Keys) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            sender,
            receiver,
            keys: keys.to_owned(),
            db_client: None,
        }
    }

    fn with_client(mut self, db_client: Database) -> Self {
        self.db_client = Some(db_client);
        self
    }

    pub async fn process_in(&mut self, backend_input: BackEndInput) -> Event {
        if let Some(db_client) = &mut self.db_client {
            backend_processing(&db_client.pool, &self.keys, backend_input).await
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
        if let Some(db_client) = &mut self.db_client {
            let pool = &db_client.pool;
            match message {
                // ---- CONFIG ----
                Message::Logout => {
                    panic!("Logout should be processed outside here")
                }
                Message::CreateAccount((profile, keys)) => {
                    tracing::info!("Creating account");
                    Event::ProfileCreated
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
                Message::RequestEvents => match DbEvent::fetch_last(pool).await {
                    Ok(last_event) => {
                        to_nostr_channel(client_sender, NostrInput::RequestEvents(last_event))
                    }
                    Err(e) => Event::Error(e.to_string()),
                },
                Message::FetchRelayResponses(event_id) => {
                    process_async_with_event(fetch_relays_responses(pool, event_id)).await
                }
                Message::AddToUnseenCount(db_contact) => {
                    process_async_with_event(add_to_unseen_count(pool, db_contact)).await
                }
                Message::ImportContacts(db_contacts) => {
                    process_async_with_event(import_contacts(&self.keys, pool, &db_contacts)).await
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
                Message::SendContactListToRelay((db_relay, list)) => to_nostr_channel(
                    client_sender,
                    NostrInput::SendContactListToRelay((db_relay, list)),
                ),
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
        database: DatabaseState,
    },
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        database: DatabaseState,
        nostr: NostrState,
    },
    OnlyNostr {
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr: NostrState,
    },
}
impl State {
    pub fn start(keys: Keys) -> Self {
        let database = DatabaseState::new(&keys);
        Self::Start { keys, database }
    }
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<Event>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub = subscription::unfold(
        id,
        State::start(keys.to_owned()),
        |state| async move {
            match state {
                State::End => iced::futures::future::pending().await,
                State::OnlyNostr { receiver: _, nostr } => {
                    if let Err(e) = nostr.logout().await {
                        tracing::error!("{}", e);
                    }
                    (Event::LoggedOut, State::End)
                }
                State::Start { keys, database } => {
                    let (sender, receiver) = mpsc::unbounded();
                    let nostr = NostrState::new(&keys).await;
                    let db_client = match Database::new(&keys.public_key().to_string()).await {
                        Ok(database) => database,
                        Err(e) => {
                            tracing::error!("Failed to init database");
                            tracing::error!("{}", e);
                            tracing::warn!("Trying again in 2 secs");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            return (Event::Disconnected, State::Start { keys, database });
                        }
                    };

                    (
                        Event::Connected(BackEndConnection::new(sender)),
                        State::Connected {
                            receiver,
                            database: database.with_client(db_client),
                            nostr,
                        },
                    )
                }
                State::Connected {
                    mut receiver,
                    mut database,
                    mut nostr,
                } => {
                    let event: Event = futures::select! {
                        message = receiver.select_next_some() => {
                            if let Message::Logout = message {
                                if let Err(e) = database.logout().await {
                                    tracing::error!("{}", e);
                                }
                                return (Event::DatabaseClosed, State::OnlyNostr { receiver, nostr });
                            } else {
                                database.process_message(&mut nostr.in_sender, message).await
                            }
                        }
                        nostr_input = nostr.in_receiver.select_next_some() => {
                            nostr.process_in(nostr_input).await
                        }
                        nostr_output = nostr.out_receiver.select_next_some() => {
                            nostr.process_out(nostr_output, &mut database.sender).await
                        }
                        backend_input = database.receiver.select_next_some() => {
                            database.process_in(backend_input).await
                        }
                        notification = nostr.notifications_stream.select_next_some() => {
                            database.process_notification(notification).await
                        },
                    };

                    (
                        event,
                        State::Connected {
                            receiver,
                            database,
                            nostr,
                        },
                    )
                }
            }
        },
    );

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

fn try_send_to_channel<T>(
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
