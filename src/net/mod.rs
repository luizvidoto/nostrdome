use crate::crypto::encrypt_local_message;
use crate::db::{Database, DbContact, DbEvent, DbMessage, MessageStatus};
use crate::error::Error;
use crate::types::ChatMessage;
use async_stream::stream;
use futures::channel::mpsc::TrySendError;
use futures::Future;
use iced::futures::stream::Fuse;
use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    Client, Contact, EventId, Filter, Keys, Kind, Relay, RelayMessage, RelayPoolNotification,
    Timestamp, Url,
};
use sqlx::SqlitePool;
use std::pin::Pin;
use std::time::Duration;

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
    SendDMTo((XOnlyPublicKey, String)),
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
    DeleteContact(XOnlyPublicKey),
    // InsertEvent(nostr_sdk::Event),
    UpdateUnseenCount(DbContact),
}

#[derive(Debug, Clone)]
pub enum DatabaseSuccessEventKind {
    RelayCreated,
    RelayUpdated,
    RelayDeleted,
    ContactCreated(DbContact),
    ContactUpdated(DbContact),
    ContactDeleted(XOnlyPublicKey),
    ContactsImported(Vec<DbContact>),
    EventInserted(DbEvent),
    NewDM((DbContact, DbMessage)),
    NewDMAndContact((DbContact, DbMessage)),
}

const IN_MEMORY: bool = true;

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

                    tracing::info!("Subscribing to filters");
                    let recv_msgs_sub = Filter::new()
                        .pubkey(keys.public_key())
                        .kind(Kind::EncryptedDirectMessage)
                        .since(Timestamp::now());
                    nostr_client.subscribe(vec![recv_msgs_sub]).await;

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
                    let event = futures::select! {
                        message = receiver.select_next_some() => {
                            let event = match message {
                                Message::UpdateUnseenCount(contact) => {
                                    process_async_fn(
                                        fetch_and_update_unseen_count(&database.pool, contact),
                                        |contact| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated(contact))
                                    ).await
                                }
                                Message::AddContact(db_contact) => {
                                    process_async_fn(
                                        DbContact::insert(&database.pool, &db_contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactCreated(db_contact.clone())),
                                    )
                                    .await
                                }
                                Message::ImportContacts(db_contacts) => {
                                    process_async_fn(
                                        DbContact::insert_batch(&database.pool, &db_contacts),
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
                                Message::DeleteContact(pubkey) => {
                                    process_async_fn(
                                        DbContact::delete(&database.pool, &pubkey),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactDeleted(pubkey)),
                                    )
                                    .await
                                }
                                Message::FetchContacts => {
                                    process_async_fn(
                                        DbContact::fetch(&database.pool, None),
                                        |contacts| Event::GotContacts(contacts),
                                    )
                                    .await
                                }

                                Message::FetchMessages (contact) => {
                                    process_async_fn(
                                        fetch_and_decrypt_chat(&keys, &database.pool, &contact.pubkey),
                                        |messages| {
                                            let chat_msgs:Vec<_> = messages
                                                .iter()
                                                .map(|msg|{
                                                    let is_from_user = &msg.from_pub == &keys.public_key();
                                                    ChatMessage::from_db_message(msg, is_from_user, &contact)
                                                })
                                                .collect();
                                            Event::GotChatMessages((contact.clone(), chat_msgs))
                                        },
                                    )
                                    .await
                                }
                                // Message::InsertEvent(event) => {
                                //     process_async_fn(
                                //         insert_event( &database.pool, event),
                                //         |event| event
                                //     ).await
                                // }
                                Message::SendDMTo((pubkey, msg)) => {
                                    process_async_fn(
                                        send_and_store_dm(&database.pool, &nostr_client, &keys, pubkey, &msg),
                                        |event_id| Event::SomeEventSuccessId(event_id),
                                    )
                                    .await
                                }
                                Message::ShowPublicKey => {
                                    let pb_key = keys.public_key();
                                    Event::GotPublicKey(pb_key)
                                }
                                // Message::ListOwnEvents => {
                                //     process_async_fn(
                                //         request_events(&nostr_client, &keys.public_key()),
                                //         |_| Event::RequestedEvents,
                                //     )
                                //     .await
                                // }
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
                                        connect_relays(&nostr_client, &keys),
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
                                        insert_event(&database.pool, event, &relay_url),
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
// async fn set_contact_list(nostr_client: &Client, contacts: Vec<Contact>) -> Result<(), Error> {
//     tracing::info!("Setting contact list on client");
//     nostr_client.set_contact_list(contacts).await?;
//     Ok(())
// }

async fn fetch_relays(nostr_client: &Client) -> Result<Vec<Relay>, Error> {
    tracing::info!("Fetching relays");
    let relays = nostr_client.relays().await;
    Ok(relays.into_iter().map(|(_url, r)| r).collect())
}
async fn add_relay(nostr_client: &Client, url: &Url) -> Result<(), Error> {
    tracing::info!("Add relay to client: {}", url);
    nostr_client.add_relay(url.as_str(), None).await?;
    Ok(())
}
async fn update_relay_db_and_client(
    _pool: &SqlitePool,
    _nostr_client: &Client,
    _url: &Url,
) -> Result<(), Error> {
    tracing::info!("Updating relay db");
    Ok(())
}

async fn connect_relay(nostr_client: &Client, relay_url: &Url) -> Result<(), Error> {
    if let Some(relay) = nostr_client
        .relays()
        .await
        .values()
        .find(|r| &r.url() == relay_url)
    {
        if let nostr_sdk::RelayStatus::Connected = relay.status().await {
            tracing::info!("Disconnecting from relay");
            nostr_client.disconnect_relay(relay_url.as_str()).await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        tracing::info!("Connecting to relay: {}", relay_url);
        nostr_client.connect_relay(relay_url.as_str()).await?;
    } else {
        tracing::info!("Relay not found on client: {}", relay_url);
    }

    Ok(())
}

async fn connect_relays(nostr_client: &Client, keys: &Keys) -> Result<(), Error> {
    tracing::info!("Adding relays to client");
    // Add relays to client
    for r in vec![
        // "wss://eden.nostr.land",
        // "wss://relay.snort.social",
        // "wss://relay.nostr.band",
        // "wss://nostr.fmt.wiz.biz",
        // "wss://relay.damus.io",
        // "wss://nostr.anchel.nl/",
        // "ws://192.168.15.119:8080"
        "ws://192.168.15.151:8080",
        // "ws://0.0.0.0:8080",
    ] {
        match nostr_client.add_relay(r, None).await {
            Ok(_) => tracing::info!("Added: {}", r),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::info!("Connecting to relays");
    nostr_client.connect().await;

    request_events(&nostr_client, &keys.public_key()).await?;

    Ok(())
}
async fn toggle_read_for_relay(nostr_client: &Client, url: &Url, read: bool) -> Result<(), Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(url) {
        relay.opts().set_read(read)
    }
    Ok(())
}
async fn toggle_write_for_relay(
    nostr_client: &Client,
    url: &Url,
    write: bool,
) -> Result<(), Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(url) {
        relay.opts().set_write(write)
    }
    Ok(())
}

async fn request_events(nostr_client: &Client, pubkey: &XOnlyPublicKey) -> Result<(), Error> {
    tracing::info!("Requesting events");
    let recv_msgs_sub = Filter::new()
        .pubkey(pubkey.to_owned())
        .kind(Kind::EncryptedDirectMessage)
        .until(Timestamp::now());
    let timeout = Duration::from_secs(10);
    nostr_client
        .req_events_of(vec![recv_msgs_sub], Some(timeout))
        .await;
    Ok(())
}

async fn send_and_store_dm(
    pool: &SqlitePool,
    nostr_client: &Client,
    keys: &Keys,
    to_pubkey: XOnlyPublicKey,
    message: &str,
) -> Result<EventId, Error> {
    tracing::info!("Sending DM to relays");
    let secret_key = keys.secret_key()?;
    // Send to relays
    let event_id = nostr_client
        .send_direct_msg(to_pubkey.clone(), message)
        .await?;

    // Encrypt the message for local storage
    let encrypted_content = encrypt_local_message(&secret_key, message)?;
    // Store the locally encrypted message in the local database
    // store_in_local_database(&locally_encrypted_message).await?;
    let db_message = DbMessage::new_local(&keys.public_key(), &to_pubkey, &encrypted_content)?;
    tracing::info!("Inserting local message");
    DbMessage::insert_message(pool, &db_message).await?;

    Ok(event_id)
}

async fn insert_event(
    pool: &SqlitePool,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::info!("Inserting event: {:?}", event);

    let mut db_event = DbEvent::from_event(event)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event.event_id = Some(row_id);

    if rows_changed == 0 {
        return Ok(Event::None);
    }

    if let Kind::EncryptedDirectMessage = db_event.kind {
        let db_message = DbMessage::from_db_event(db_event, Some(relay_url.to_owned()))?;
        let contact_result = DbContact::fetch_one(pool, &db_message.from_pub).await;

        tracing::info!("Inserting external message");
        DbMessage::insert_message(pool, &db_message).await?;

        let database_success_event_kind = match contact_result {
            Ok(Some(contact)) => {
                tracing::info!("Have contact: {:?}", contact);
                DatabaseSuccessEventKind::NewDM((contact, db_message.clone()))
            }
            Ok(None) => {
                let mut db_contact = DbContact::new(&db_message.from_pub);
                db_contact.unseen_messages = 1;
                DbContact::insert(pool, &db_contact).await?;
                DatabaseSuccessEventKind::NewDMAndContact((db_contact, db_message.clone()))
            }
            Err(e) => return Err(e),
        };

        return Ok(Event::DatabaseSuccessEvent(database_success_event_kind));
    }

    Ok(Event::DatabaseSuccessEvent(
        DatabaseSuccessEventKind::EventInserted(db_event),
    ))
}

async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    contact: &XOnlyPublicKey,
) -> Result<Vec<DbMessage>, Error> {
    tracing::info!("Fetching chat messages");
    let secret_key = keys.secret_key()?;
    let own_pubkey = keys.public_key();
    let mut messages = DbMessage::fetch_chat(pool, &own_pubkey, contact).await?;

    tracing::info!("Updating unseen messages to marked as seen");
    for m in messages.iter_mut().filter(|m| m.is_unseen()) {
        m.status = MessageStatus::Seen;
        DbMessage::update_message_status(pool, m).await?;
    }

    tracing::info!("Decrypting messages");
    for m in &mut messages {
        m.decrypt_message(&secret_key, &own_pubkey);
    }

    Ok(messages)
}
async fn fetch_and_update_unseen_count(
    pool: &SqlitePool,
    mut contact: DbContact,
) -> Result<DbContact, Error> {
    let count = DbMessage::fetch_unseen_count(pool, &contact.pubkey).await?;
    contact.unseen_messages = count;
    DbContact::update(pool, &contact).await?;
    Ok(contact.clone())
}
