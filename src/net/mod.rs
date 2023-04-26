use crate::db::{Database, DbChat, DbContact, DbEvent, DbMessage, MessageStatus};
use crate::error::Error;
use crate::net::contact::{insert_batch_of_contacts, insert_contact};
use crate::net::event::insert_event_with_tt;
use crate::net::relay::{
    add_relay, connect_relay, connect_relays, fetch_relays, toggle_read_for_relay,
    toggle_write_for_relay, update_relay_db_and_client,
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
    Client, Contact, EventBuilder, EventId, Keys, Relay, RelayMessage, RelayPoolNotification, Url,
};
use sqlx::SqlitePool;
use std::pin::Pin;
use std::time::Duration;

mod contact;
mod event;
mod relay;

#[derive(Debug, Clone)]
pub struct BackEndConnection(mpsc::UnboundedSender<Message>);
impl BackEndConnection {
    pub fn send(&mut self, message: Message) {
        // TODO
        // error: send failed because receiver is gone
        if let Err(e) = self.0.unbounded_send(message).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
    pub fn try_send(&mut self, message: Message) -> Result<(), TrySendError<Message>> {
        self.0.unbounded_send(message)
    }
}
pub enum State {
    Disconnected {
        keys: Keys,
        in_memory: bool,
    },
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Client,
        database: Database,
        keys: Keys,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
    },
}
#[derive(Debug, Clone)]
pub enum Event {
    /// Event triggered when a connection to the back-end is established
    Connected(BackEndConnection),

    /// Event triggered when the connection to the back-end is lost
    Disconnected,

    /// Event triggered when an error occurs
    Error(String),

    /// NOSTR CLIENT EVENTS
    /// Event triggered when relays are connected
    RelaysConnected,

    /// Event triggered when a list of relays is received
    GotRelays(Vec<Relay>),

    /// Event triggered when a list of own events is received
    GotOwnEvents(Vec<nostr_sdk::Event>),

    /// Event triggered when a public key is received
    GotPublicKey(XOnlyPublicKey),

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
    DatabaseSuccessEvent(DatabaseSuccessEventKind),

    /// Event triggered when a list of chat messages is received
    GotChatMessages((DbContact, Vec<ChatMessage>)),

    /// Event triggered when a list of contacts is received
    GotContacts(Vec<DbContact>),

    RequestedEvents,

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
    // ListOwnEvents,
    FetchRelays,
    AddRelay(Url),
    UpdateRelay(Url),
    DeleteRelay(Url),
    ToggleRelayRead((Url, bool)),
    ToggleRelayWrite((Url, bool)),

    // DATABASE MESSAGES
    FetchMessages(DbContact),
    AddContact(DbContact),
    ImportContacts(Vec<DbContact>),
    FetchContacts,
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    // InsertEvent(nostr_sdk::Event),
    // UpdateUnseenCount(DbContact),
    AddToUnseenCount(DbContact),
}

#[derive(Debug, Clone)]
pub enum DatabaseSuccessEventKind {
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
    // SentDM((EventId, DbContact, ChatMessage)),
}

pub fn backend_connect(keys: &Keys) -> Subscription<Event> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    subscription::unfold(
        id,
        State::Disconnected {
            keys: keys.clone(),
            in_memory: IN_MEMORY,
        },
        |state| async move {
            match state {
                State::Disconnected { keys, in_memory } => {
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

                    let database = match Database::new(in_memory, &keys.public_key().to_string())
                        .await
                    {
                        Ok(database) => database,
                        Err(e) => {
                            tracing::error!("Failed to init database");
                            tracing::error!("{}", e);
                            tracing::warn!("Trying again in 2 secs");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            return (Event::Disconnected, State::Disconnected { keys, in_memory });
                        }
                    };

                    (
                        Event::Connected(BackEndConnection(sender)),
                        State::Connected {
                            database,
                            receiver,
                            nostr_client,
                            keys,
                            notifications_stream,
                        },
                    )
                }
                State::Connected {
                    database,
                    mut receiver,
                    nostr_client,
                    keys,
                    mut notifications_stream,
                } => {
                    // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    let event = futures::select! {
                        message = receiver.select_next_some() => {
                            let event = match message {
                                Message::AddToUnseenCount(db_contact) => {
                                    process_async_fn(
                                        add_to_unseen_count(&database.pool, db_contact),
                                        |contact| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated(contact))
                                    ).await
                                }
                                // Message::UpdateUnseenCount(contact) => {
                                //     process_async_fn(
                                //         update_unseen_count(&database.pool, &keys, contact),
                                //         |contact| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated(contact))
                                //     ).await
                                // }
                                Message::AddContact(db_contact) => {
                                    process_async_fn(
                                        insert_contact(&keys, &database.pool, &db_contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactCreated(db_contact.clone())),
                                    )
                                    .await
                                }
                                Message::ImportContacts(db_contacts) => {
                                    process_async_fn(
                                        insert_batch_of_contacts(&keys, &database.pool, &db_contacts),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactsImported(db_contacts.clone())),
                                    )
                                    .await
                                }
                                Message::UpdateContact(db_contact) => {
                                    process_async_fn(
                                        DbContact::update(&database.pool, &db_contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated(db_contact.clone())),
                                    )
                                    .await
                                }
                                Message::DeleteContact(contact) => {
                                    process_async_fn(
                                        DbContact::delete(&database.pool, &contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactDeleted(contact.clone())),
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
                                Message::SendDMTo((contact, msg)) => {
                                    process_async_fn(
                                        send_and_store_dm(&database.pool, &nostr_client, &keys, &contact, &msg),
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
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayDeleted),
                                    )
                                    .await
                                }
                                Message::AddRelay(url) => {
                                    process_async_fn(
                                        add_relay(&nostr_client, &url),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayCreated),
                                    )
                                    .await
                                }
                                Message::UpdateRelay(url) => {
                                    process_async_fn(
                                        update_relay_db_and_client(&database.pool, &nostr_client, &url),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated),
                                    )
                                    .await
                                }
                                Message::ConnectRelays => {
                                    process_async_fn(
                                        connect_relays(&database.pool,&nostr_client, &keys),
                                        |_| Event::RelaysConnected,
                                    )
                                    .await
                                }
                                Message::ToggleRelayRead((url, read)) => {
                                    process_async_fn(
                                        toggle_read_for_relay(&nostr_client, &url, read),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated)
                                    ).await
                                }
                                Message::ToggleRelayWrite((url, write)) => {
                                    process_async_fn(
                                        toggle_write_for_relay(&nostr_client, &url, write),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated)
                                    ).await
                                }
                            };

                            event
                        }
                        notification = notifications_stream.select_next_some() => {
                            let event = match notification {
                                RelayPoolNotification::Event(relay_url, event) => {
                                    process_async_fn(
                                        insert_event_with_tt(&database.pool, &keys, event, &relay_url),
                                        |event| event
                                    ).await
                                },
                                RelayPoolNotification::Message(_url, relay_msg) => {
                                    Event::RelayMessage(relay_msg)
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
                        State::Connected {
                            database,
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

async fn send_and_store_dm(
    pool: &SqlitePool,
    nostr_client: &Client,
    keys: &Keys,
    db_contact: &DbContact,
    message: &str,
) -> Result<Event, Error> {
    tracing::info!("Sending DM to relays");
    let mut has_event: Option<(nostr_sdk::Event, Url)> = None;
    let builder =
        EventBuilder::new_encrypted_direct_msg(&keys, db_contact.pubkey().to_owned(), message)?;
    let event = builder.to_event(keys)?;

    for (url, relay) in nostr_client.relays().await {
        if !relay.opts().write() {
            // return Err(Error::WriteActionsDisabled(url.clone()))
            tracing::error!("{}", Error::WriteActionsDisabled(url.to_string()));
            continue;
        }

        if let Ok(_id) = nostr_client.send_event_to(url.clone(), event.clone()).await {
            has_event = Some((event.clone(), url.clone()));
        }
    }

    if let Some((event, relay_url)) = has_event {
        Ok(insert_event_with_tt(pool, keys, event, &relay_url).await?)
    } else {
        Err(Error::NoRelayToWrite)
    }
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

const IN_MEMORY: bool = false;
