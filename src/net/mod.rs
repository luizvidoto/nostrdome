use crate::crypto::{decrypt_local_message, encrypt_local_message};
use crate::db::{Database, DbContact, DbEvent};
use crate::error::Error;
use crate::types::ChatMessage;
use async_stream::stream;
use futures::channel::mpsc::TrySendError;
use futures::Future;
use iced::futures::stream::Fuse;
use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::prelude::decrypt;
use nostr_sdk::secp256k1::{SecretKey, XOnlyPublicKey};
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
    GotMessages(Vec<ChatMessage>),

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
    ListOwnEvents,
    FetchRelays,
    AddRelay(Url),
    UpdateRelay(Url),
    DeleteRelay(Url),
    ToggleRelayRead((Url, bool)),
    ToggleRelayWrite((Url, bool)),

    // DATABASE MESSAGES
    FetchMessages(XOnlyPublicKey),
    AddContact(DbContact),
    ImportContacts(Vec<DbContact>),
    SetContactList(Vec<Contact>),
    FetchContacts,
    UpdateContact(DbContact),
    DeleteContact(XOnlyPublicKey),
    InsertEvent(nostr_sdk::Event),
}

#[derive(Debug, Clone)]
pub enum DatabaseSuccessEventKind {
    RelayCreated,
    RelayUpdated,
    RelayDeleted,
    ContactCreated,
    ContactUpdated,
    ContactDeleted,
    EventInserted(nostr_sdk::Event),
    NewDM(ChatMessage),
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

                    // tracing::info!("Connecting to relays");
                    // nostr_client.connect().await;

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
                    futures::select! {
                        message = receiver.select_next_some() => {
                            let event = match message {
                                Message::AddContact(db_contact) => {
                                    process_async_fn(
                                        DbContact::insert(&database.pool, &db_contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactCreated),
                                    )
                                    .await
                                }
                                Message::ImportContacts(db_contacts) => {
                                    process_async_fn(
                                        DbContact::insert_batch(&database.pool, &db_contacts),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactCreated),
                                    )
                                    .await
                                }
                                Message::UpdateContact(db_contact) => {
                                    process_async_fn(
                                        DbContact::update(&database.pool, &db_contact),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated),
                                    )
                                    .await
                                }
                                Message::DeleteContact(pubkey) => {
                                    process_async_fn(
                                        DbContact::delete(&database.pool, &pubkey),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactDeleted),
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
                                Message::SetContactList(contacts) => {
                                    process_async_fn(
                                        set_contact_list(&nostr_client, contacts),
                                        |_|Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::ContactUpdated)
                                    ).await
                                }
                                Message::FetchMessages (contact) => {
                                    process_async_fn(
                                        DbEvent::fetch(
                                            &database.pool,
                                            Some(&format!(
                                                "pubkey = '{}' OR pubkey = '{}'",
                                                &keys.public_key(),
                                                contact
                                            )),
                                        ),
                                        |events| {
                                            let secret_key = match keys.secret_key() {
                                                Ok(sk) => sk,
                                                Err(e) => return Event::Error(e.to_string()),
                                            };
                                            let messages: Vec<_> = events
                                                .iter()
                                                .filter_map(|ev| {
                                                    if let Kind::EncryptedDirectMessage = ev.kind {
                                                        match decrypt(&secret_key, &ev.pubkey, &ev.content) {
                                                            Ok(msg) => Some(ChatMessage::from_event(ev, msg)),
                                                            Err(_e) => Some(ChatMessage::from_event(ev, &ev.content)),
                                                            // Err(_e) => None,
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect();
                                            Event::GotMessages(messages)
                                        },

                                    )
                                    .await
                                }
                                Message::InsertEvent(event) => {
                                    process_async_fn(
                                        DbEvent::insert(&database.pool, DbEvent::from(&event)),
                                        |rows_changed| {
                                            if rows_changed > 0 {
                                                if let Kind::EncryptedDirectMessage = event.kind {
                                                    let secret_key = match keys.secret_key() {
                                                        Ok(sk) => sk,
                                                        Err(e) => return Event::Error(e.to_string()),
                                                    };
                                                    match decrypt(&secret_key, &event.pubkey, &event.content) {
                                                        Ok(msg) => Event::DatabaseSuccessEvent(
                                                            DatabaseSuccessEventKind::NewDM(ChatMessage::from_event(
                                                                &event, msg,
                                                            )),
                                                        ),
                                                        Err(_e) => Event::DatabaseSuccessEvent(
                                                            DatabaseSuccessEventKind::NewDM(ChatMessage::from_event(
                                                                &event,
                                                                &event.content,
                                                            )),
                                                        ),
                                                        // Err(e) => (Event::Error(e.to_string()), State::Connected {database, receiver, nostr_client, keys, notifications_stream}),
                                                    }
                                                } else {
                                                    Event::DatabaseSuccessEvent(
                                                        DatabaseSuccessEventKind::EventInserted(event.clone()),
                                                    )
                                                }
                                            } else {
                                                Event::None
                                            }
                                        },

                                    )
                                    .await
                                }
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
                                Message::ListOwnEvents => {
                                    process_async_fn(
                                        request_events(&nostr_client, &keys.public_key()),
                                        |_| Event::RequestedEvents,
                                    )
                                    .await
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
                                        connect_relays(&nostr_client),
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
                            (event, State::Connected {database, receiver, nostr_client, keys, notifications_stream})
                        }
                        notification = notifications_stream.select_next_some() => {
                            let event = match notification {
                                RelayPoolNotification::Event(_url, event) => {
                                    // tracing::info!("url: {}", _url);
                                    // tracing::info!("event: {:?}", event);
                                    Event::NostrEvent(event)
                                    // TODO:
                                    // front end receives and says to insert event
                                    // do it all from here
                                },
                                RelayPoolNotification::Message(_url, relay_msg) => {
                                    Event::RelayMessage(relay_msg)
                                }
                                RelayPoolNotification::Shutdown => {
                                    Event::Shutdown
                                }
                            };
                            (event, State::Connected {database, receiver, nostr_client, keys, notifications_stream})
                        },
                    }
                }
            }
        },
    )
}

async fn _fetch_contacts_from_relays(nostr_client: &Client) -> Result<Vec<Contact>, Error> {
    let contacts = nostr_client
        .get_contact_list(Some(Duration::from_secs(10)))
        .await?;
    Ok(contacts)
}
async fn set_contact_list(nostr_client: &Client, contacts: Vec<Contact>) -> Result<(), Error> {
    nostr_client.set_contact_list(contacts).await?;
    Ok(())
}

async fn fetch_relays(nostr_client: &Client) -> Result<Vec<Relay>, Error> {
    let relays = nostr_client.relays().await;

    Ok(relays.into_iter().map(|(_url, r)| r).collect())
}
async fn add_relay(nostr_client: &Client, url: &Url) -> Result<(), Error> {
    nostr_client.add_relay(url.as_str(), None).await?;
    Ok(())
}
async fn update_relay_db_and_client(
    _pool: &SqlitePool,
    _nostr_client: &Client,
    _url: &Url,
) -> Result<(), Error> {
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
            nostr_client.disconnect_relay(relay_url.as_str()).await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        nostr_client.connect_relay(relay_url.as_str()).await?;
    }

    Ok(())
}

async fn connect_relays(nostr_client: &Client) -> Result<(), Error> {
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
async fn request_events(nostr_client: &Client, pubkey: &XOnlyPublicKey) -> Result<(), Error> {
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
    pubkey: XOnlyPublicKey,
    message: &str,
) -> Result<EventId, Error> {
    let secret_key = keys.secret_key()?;
    let event_id = nostr_client.send_direct_msg(pubkey, message).await?;

    // Encrypt the message for local storage
    let locally_encrypted_message = encrypt_local_message(&secret_key, message)?;
    // Store the locally encrypted message in the local database
    store_in_local_database(&locally_encrypted_message).await?;

    Ok(event_id)
}

fn read_message_from_database(message_id: i64, secret_key: &SecretKey) -> Result<String, Error> {
    // Fetch the encrypted message from the local database
    let encrypted_message = get_message_from_database(message_id)?;

    // Decrypt the locally encrypted message
    let decrypted_message = decrypt_local_message(secret_key, &encrypted_message)?;

    Ok(decrypted_message)
}
