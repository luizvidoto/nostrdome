use crate::db::{Database, DbChat, DbContact, DbEvent, DbMessage, DbRelayResponse, MessageStatus};
use crate::error::Error;
use crate::net::contact::{insert_batch_of_contacts, insert_contact};
use crate::net::event::send_dm;
use crate::net::relay::{
    add_relay, connect_relay, connect_relays, fetch_relays, fetch_relays_urls,
    toggle_read_for_relay, toggle_write_for_relay, update_relay_db_and_client,
};
use crate::types::ChatMessage;
use async_stream::stream;
use futures::channel::mpsc::TrySendError;
use futures::Future;
use iced::futures::stream::Fuse;
use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    Client, Contact, EventId, Keys, Kind, Metadata, Relay, RelayMessage, RelayPoolNotification, Url,
};
use sqlx::SqlitePool;
use std::pin::Pin;
use std::str::FromStr;
use std::time::Duration;

mod contact;
mod event;
mod relay;

pub trait Connection {
    fn sender(&self) -> &mpsc::UnboundedSender<Message>;

    fn send(&mut self, message: Message) {
        if let Err(e) = self
            .sender()
            .unbounded_send(message)
            .map_err(|e| e.to_string())
        {
            tracing::error!("{}", e);
        }
    }

    fn try_send(&mut self, message: Message) -> Result<(), TrySendError<Message>> {
        self.sender().unbounded_send(message)
    }
}

#[derive(Debug, Clone)]
pub struct DBConnection(mpsc::UnboundedSender<Message>);
impl Connection for DBConnection {
    fn sender(&self) -> &mpsc::UnboundedSender<Message> {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct NostrConnection(mpsc::UnboundedSender<Message>);
impl Connection for NostrConnection {
    fn sender(&self) -> &mpsc::UnboundedSender<Message> {
        &self.0
    }
}

pub enum DatabaseState {
    Disconnected {
        keys: Keys,
        in_memory: bool,
    },
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        database: Database,
        keys: Keys,
    },
}

pub enum NostrClientState {
    Disconnected {
        keys: Keys,
    },
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Client,
        keys: Keys,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
    },
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<Event>> {
    let database_sub = database_connect(keys);
    let nostr_client_sub = nostr_client_connect(keys);

    vec![database_sub, nostr_client_sub]
}

pub fn database_connect(keys: &Keys) -> Subscription<Event> {
    struct Backend;
    let id = std::any::TypeId::of::<Database>();

    subscription::unfold(
        id,
        DatabaseState::Disconnected {
            keys: keys.clone(),
            in_memory: IN_MEMORY,
        },
        |state| async move {
            match state {
                DatabaseState::Disconnected { keys, in_memory } => {
                    let (sender, receiver) = mpsc::unbounded();

                    let database =
                        match Database::new(in_memory, &keys.public_key().to_string()).await {
                            Ok(database) => database,
                            Err(e) => {
                                tracing::error!("Failed to init database");
                                tracing::error!("{}", e);
                                tracing::warn!("Trying again in 2 secs");
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                return (
                                    Event::Disconnected,
                                    DatabaseState::Disconnected { keys, in_memory },
                                );
                            }
                        };

                    (
                        Event::DbConnected(DBConnection(sender)),
                        DatabaseState::Connected {
                            database,
                            receiver,
                            keys,
                        },
                    )
                }
                DatabaseState::Connected {
                    database,
                    mut receiver,
                    keys,
                } => {
                    let event = futures::select! {
                        message = receiver.select_next_some() => {
                            let event = match message {
                                Message::FetchRelayResponses(event_id) => {
                                    process_async_fn(
                                        fetch_relays_responses(&database.pool, event_id),
                                        |responses| Event::GotRelayResponses(responses)
                                    ).await
                                }
                                Message::AddToUnseenCount(db_contact) => {
                                    process_async_fn(
                                        add_to_unseen_count(&database.pool, db_contact),
                                        |contact| Event::DBSuccessEvent(SuccessKind::ContactUpdated(contact))
                                    ).await
                                }
                                Message::AddContact(db_contact) => {
                                    process_async_fn(
                                        insert_contact(&keys, &database.pool, &db_contact),
                                        |_| Event::DBSuccessEvent(SuccessKind::ContactCreated(db_contact.clone())),
                                    )
                                    .await
                                }
                                Message::ImportContacts(db_contacts) => {
                                    process_async_fn(
                                        insert_batch_of_contacts(&keys, &database.pool, &db_contacts),
                                        |_| Event::DBSuccessEvent(SuccessKind::ContactsImported(db_contacts.clone())),
                                    )
                                    .await
                                }
                                Message::UpdateContact(db_contact) => {
                                    process_async_fn(
                                        DbContact::update(&database.pool, &db_contact),
                                        |_| Event::DBSuccessEvent(SuccessKind::ContactUpdated(db_contact.clone())),
                                    )
                                    .await
                                }
                                Message::DeleteContact(contact) => {
                                    process_async_fn(
                                        DbContact::delete(&database.pool, &contact),
                                        |_| Event::DBSuccessEvent(SuccessKind::ContactDeleted(contact.clone())),
                                    )
                                    .await
                                }
                                Message::FetchContacts => {
                                    process_async_fn(
                                        DbContact::fetch(&database.pool),
                                        |contacts| Event::GotContacts(contacts),
                                    )
                                    .await
                                }
                                Message::FetchMessages (contact) => {
                                    process_async_fn(
                                        fetch_and_decrypt_chat(&keys, &database.pool, contact),
                                        |(contact, messages)| Event::GotChatMessages((contact, messages)),
                                    )
                                    .await
                                }
                                _ => {
                                    Event::None //TODO: remove this
                                }
                            };

                            event
                        }
                    };
                    (
                        event,
                        DatabaseState::Connected {
                            database,
                            receiver,
                            keys,
                        },
                    )
                }
            }
        },
    )
}

pub fn nostr_client_connect(keys: &Keys) -> Subscription<Event> {
    struct NostrClient;
    let id = std::any::TypeId::of::<NostrClient>();

    subscription::unfold(
        id,
        NostrClientState::Disconnected { keys: keys.clone() },
        |state| async move {
            match state {
                NostrClientState::Disconnected { keys } => {
                    // Create new client
                    tracing::info!("Creating Nostr Client");
                    let nostr_client = Client::new(&keys);

                    let mut notifications = nostr_client.notifications();
                    let notifications_stream = stream! {
                        while let Ok(notification) = notifications.recv().await {
                            yield notification;
                        }
                    }
                    .boxed()
                    .fuse();

                    let (sender, receiver) = mpsc::unbounded();

                    (
                        Event::NostrConnected(NostrConnection(sender)),
                        NostrClientState::Connected {
                            receiver,
                            nostr_client,
                            keys,
                            notifications_stream,
                        },
                    )
                }
                NostrClientState::Connected {
                    mut receiver,
                    nostr_client,
                    keys,
                    mut notifications_stream,
                } => {
                    // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    let event = futures::select! {
                        message = receiver.select_next_some() => {
                            let event = match message {
                                Message::CreateChannel => {
                                    process_async_fn(
                                        create_channel(&nostr_client),
                                        |event| event
                                    ).await
                                }
                                Message::SendDMTo((contact, msg)) => {
                                    process_async_fn(
                                        send_dm(&nostr_client, &keys, &contact, &msg),
                                        |event| event,
                                    )
                                    .await
                                }
                                Message::ShowPublicKey => {
                                    let pb_key = keys.public_key();
                                    Event::GotPublicKey(pb_key)
                                }
                                Message::FetchRelays => {
                                    process_async_fn(
                                        fetch_relays(&nostr_client),
                                        |relays| Event::GotRelays(relays),
                                    )
                                    .await
                                }
                                Message::FetchRelaysUrls => {
                                    process_async_fn(
                                        fetch_relays_urls(&nostr_client),
                                        |relays| Event::GotRelaysUrls(relays),
                                    )
                                    .await
                                }
                                Message::ConnectToRelay(relay_url) => {
                                    process_async_fn(
                                        connect_relay(&nostr_client, &relay_url),
                                        |_| Event::RelayConnected(relay_url.clone())
                                    ).await
                                }
                                Message::DeleteRelay(relay_url) => {
                                    process_async_fn(
                                        // DbRelay::delete(&database.pool, &relay_url),
                                        nostr_client.remove_relay(relay_url.as_str()),
                                        |_| Event::DBSuccessEvent(SuccessKind::RelayDeleted),
                                    )
                                    .await
                                }
                                Message::AddRelay(url) => {
                                    println!("Message::AddRelay");
                                    process_async_fn(
                                        add_relay(&nostr_client, &url),
                                        |_| Event::DBSuccessEvent(SuccessKind::RelayCreated),
                                    )
                                    .await
                                }
                                Message::UpdateRelay(url) => {
                                    process_async_fn(
                                        update_relay_db_and_client(&nostr_client, &url),
                                        |_| Event::DBSuccessEvent(SuccessKind::RelayUpdated),
                                    )
                                    .await
                                }
                                Message::ConnectRelays => {
                                    process_async_fn(
                                        connect_relays(&nostr_client, &keys, 0), //TODO: fetch from db
                                        |_| Event::RelaysConnected,
                                    )
                                    .await
                                }
                                Message::ToggleRelayRead((url, read)) => {
                                    process_async_fn(
                                        toggle_read_for_relay(&nostr_client, &url, read),
                                        |_| Event::DBSuccessEvent(SuccessKind::RelayUpdated)
                                    ).await
                                }
                                Message::ToggleRelayWrite((url, write)) => {
                                    process_async_fn(
                                        toggle_write_for_relay(&nostr_client, &url, write),
                                        |_| Event::DBSuccessEvent(SuccessKind::RelayUpdated)
                                    ).await
                                },
                                _ => Event::None //TODO: remove this
                            };

                            event
                        }
                        notification = notifications_stream.select_next_some() => {
                            let event = match notification {
                                RelayPoolNotification::Event(relay_url, event) => {
                                    // println!("*** NOTIFICATION ***");
                                    // dbg!(&event);
                                    // process_async_fn(
                                    //     received_event(&database.pool, &keys, event, &relay_url),
                                    //     |event| event
                                    // ).await
                                    Event::ReceivedEvent((relay_url, event))
                                },
                                RelayPoolNotification::Message(relay_url, msg) => {
                                    // process_async_fn(
                                    //     on_relay_message(&database.pool, &relay_url, &msg),
                                    //     |event| event
                                    // ).await
                                    Event::ReceivedRelayMessage((relay_url, msg))
                                }
                                RelayPoolNotification::Shutdown => {
                                    Event::Shutdown
                                }
                            };

                            event
                        },
                    };
                    (
                        event,
                        NostrClientState::Connected {
                            receiver,
                            nostr_client,
                            keys,
                            notifications_stream,
                        },
                    )
                }
            }
        },
    )
}

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

async fn _fetch_contacts_from_relays(nostr_client: &Client) -> Result<Vec<Contact>, Error> {
    let contacts = nostr_client
        .get_contact_list(Some(Duration::from_secs(10)))
        .await?;
    Ok(contacts)
}

async fn add_to_unseen_count(
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<DbContact, Error> {
    db_contact = DbContact::add_to_unseen_count(pool, &mut db_contact).await?;
    Ok(db_contact)
}

async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<(DbContact, Vec<ChatMessage>), Error> {
    tracing::info!("Fetching chat messages");
    let own_pubkey = keys.public_key();
    let chat = DbChat::new(&own_pubkey, db_contact.pubkey());
    let mut db_messages = chat.fetch_chat(pool).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for m in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        m.update_status(MessageStatus::Seen);
        DbMessage::update_message_status(pool, m).await?;
    }

    tracing::info!("Decrypting messages");
    for m in &mut db_messages {
        let content = m.decrypt_message(keys)?;
        let is_from_user = m.im_author(&keys.public_key());
        let chat_message = ChatMessage::from_db_message(&m, is_from_user, &db_contact, &content)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok((db_contact, chat_messages))
}

async fn on_relay_message(
    pool: &SqlitePool,
    relay_url: &Url,
    relay_message: &RelayMessage,
) -> Result<Event, Error> {
    let event = match relay_message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message,
        } => {
            let mut db_event = DbEvent::fetch_one(pool, event_hash)
                .await?
                .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
            let mut db_message = None;

            if !db_event.confirmed {
                db_event = DbEvent::confirm_event(pool, db_event).await?;

                if let Kind::EncryptedDirectMessage = db_event.kind {
                    db_message = if let Some(db_message) =
                        DbMessage::fetch_one(pool, db_event.event_id()?).await?
                    {
                        let confirmed_db_message =
                            DbMessage::confirm_message(pool, db_message).await?;
                        Some(confirmed_db_message)
                    } else {
                        None
                    };
                }
            }

            let mut relay_response = DbRelayResponse::from_response(
                *status,
                db_event.event_id()?,
                event_hash,
                relay_url,
                message,
            );
            let id = DbRelayResponse::insert(pool, &relay_response).await?;
            relay_response = relay_response.with_id(id);
            Event::DBSuccessEvent(SuccessKind::UpdateWithRelayResponse {
                relay_response,
                db_event,
                db_message,
            })
        }
        other => Event::RelayMessage(other.to_owned()),
    };

    Ok(event)
}

async fn fetch_relays_responses(
    pool: &SqlitePool,
    event_id: i64,
) -> Result<Vec<DbRelayResponse>, Error> {
    let responses = DbRelayResponse::fetch_by_event(pool, event_id).await?;

    Ok(responses)
}

async fn _send_contact_list(client: &Client, list: &[DbContact]) -> Result<Event, Error> {
    let c_list: Vec<_> = list.iter().map(|c| c.into()).collect();
    let _event_id = client.set_contact_list(c_list).await?;

    Ok(Event::None)
}

async fn create_channel(client: &Client) -> Result<Event, Error> {
    let metadata = Metadata::new()
        .about("Channel about cars")
        .display_name("Best Cars")
        .name("Best Cars")
        .banner(Url::from_str("https://picsum.photos/seed/picsum/800/300")?)
        .picture(Url::from_str("https://picsum.photos/seed/picsum/200/300")?)
        .website(Url::from_str("https://picsum.photos/seed/picsum/200/300")?);

    let event_id = client.new_channel(metadata).await?;
    Ok(Event::ChannelCreated(event_id))
}

#[derive(Debug, Clone)]
pub enum Event {
    LocalPendingEvent(DbEvent),
    InsertPendingEvent(nostr_sdk::Event),
    ReceivedEvent((Url, nostr_sdk::Event)),
    ReceivedRelayMessage((Url, nostr_sdk::RelayMessage)),

    /// Event triggered when a connection to the back-end is established
    DbConnected(DBConnection),
    NostrConnected(NostrConnection),

    /// Event triggered when the connection to the back-end is lost
    Disconnected,

    /// Event triggered when an error occurs
    Error(String),

    /// NOSTR CLIENT EVENTS
    /// Event triggered when relays are connected
    RelaysConnected,

    /// Event triggered when a list of relays is received
    GotRelays(Vec<Relay>),

    GotRelaysUrls(Vec<nostr_sdk::Url>),

    /// Event triggered when a list of own events is received
    GotOwnEvents(Vec<nostr_sdk::Event>),

    /// Event triggered when a public key is received
    GotPublicKey(XOnlyPublicKey),

    GotRelayResponses(Vec<DbRelayResponse>),

    /// Event triggered when an event is successfully processed
    SomeEventSuccessId(EventId),

    /// Event triggered when an event string is received
    GotEvent(String),

    /// Event triggered when a relay is removed
    RelayRemoved(String),

    /// Event triggered when a Nostr event is received
    NostrEvent(nostr_sdk::Event),

    /// Event triggered when a relay message is received
    RelayMessage(RelayMessage),

    /// Event triggered when the system is shutting down
    Shutdown,

    /// Event triggered when a relay is connected
    RelayConnected(Url),

    /// Event triggered when a relay is updated
    UpdatedRelay,

    /// DATABASE EVENTS
    /// Event triggered when a database operation is successful
    DBSuccessEvent(SuccessKind),

    /// Event triggered when a list of chat messages is received
    GotChatMessages((DbContact, Vec<ChatMessage>)),

    /// Event triggered when a list of contacts is received
    GotContacts(Vec<DbContact>),

    RequestedEvents,

    ChannelCreated(EventId),

    /// Event triggered when no event is to be processed
    None,
}

#[derive(Debug, Clone)]
pub enum Message {
    // NOSTR CLIENT MESSAGES
    ConnectRelays,
    ConnectToRelay(Url),
    SendDMTo((DbContact, String)),
    ShowPublicKey,
    FetchRelays,
    FetchRelaysUrls,
    AddRelay(Url),
    UpdateRelay(Url),
    DeleteRelay(Url),
    ToggleRelayRead((Url, bool)),
    ToggleRelayWrite((Url, bool)),
    SendContactListToRelay(Url),

    // DATABASE MESSAGES
    FetchRelayResponses(i64),
    FetchMessages(DbContact),
    AddContact(DbContact),
    ImportContacts(Vec<DbContact>),
    FetchContacts,
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    AddToUnseenCount(DbContact),
    CreateChannel,
}

#[derive(Debug, Clone)]
pub enum SuccessKind {
    RelayCreated,
    RelayUpdated,
    RelayDeleted,
    ContactCreated(DbContact),
    ContactUpdated(DbContact),
    ContactDeleted(DbContact),
    ContactsImported(Vec<DbContact>),
    EventInserted(DbEvent),
    ReceivedDM((DbContact, ChatMessage)),
    NewDMAndContact((DbContact, ChatMessage)),
    UpdateWithRelayResponse {
        relay_response: DbRelayResponse,
        db_event: DbEvent,
        db_message: Option<DbMessage>,
    },
}

const IN_MEMORY: bool = false;
