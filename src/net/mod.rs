use crate::db::{Database, DbChat, DbContact, DbEvent, DbMessage, MessageStatus};
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
    Client, Contact, EventBuilder, EventId, Filter, Keys, Kind, Relay, RelayMessage,
    RelayPoolNotification, Timestamp, Url,
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
                                        insert_event(&database.pool, &keys, event, &relay_url),
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
        tracing::warn!("Relay not found on client: {}", relay_url);
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

    request_events(&nostr_client, &keys.public_key(), &keys.public_key()).await?;

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

async fn request_events(
    nostr_client: &Client,
    own_pubkey: &XOnlyPublicKey,
    pubkey: &XOnlyPublicKey,
) -> Result<(), Error> {
    tracing::info!("Requesting events");
    let sent_msgs_sub = Filter::new()
        .author(own_pubkey.to_string())
        .kind(Kind::EncryptedDirectMessage)
        .until(Timestamp::now());
    let recv_msgs_sub = Filter::new()
        .pubkey(pubkey.to_owned())
        .kind(Kind::EncryptedDirectMessage)
        .until(Timestamp::now());
    let timeout = Duration::from_secs(10);
    nostr_client
        .req_events_of(vec![sent_msgs_sub, recv_msgs_sub], Some(timeout))
        .await;
    Ok(())
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
        Ok(insert_event(pool, keys, event, &relay_url).await?)
        // tracing::info!("Encrypting local message");
        // let db_message = DbMessage::new_local(&keys, db_contact.pubkey(), message)?;

        // tracing::info!("Inserting local message");
        // let msg_id = DbMessage::insert_message(pool, &db_message).await?;
        // let db_message = db_message.with_id(msg_id);

        // let chat_message = ChatMessage::from_db_message(&db_message, true, &db_contact, message)?;
        // DbContact::new_message(pool, &mut db_contact, &chat_message).await?;
        // Ok((event_id, db_contact, chat_message))
    } else {
        Err(Error::NoRelayToWrite)
    }
}

async fn insert_event(
    pool: &SqlitePool,
    keys: &Keys,
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
        let database_success_event_kind =
            insert_encrypted_dm(pool, keys, db_event, relay_url).await?;
        return Ok(Event::DatabaseSuccessEvent(database_success_event_kind));
    }

    Ok(Event::DatabaseSuccessEvent(
        DatabaseSuccessEventKind::EventInserted(db_event),
    ))
}

async fn insert_encrypted_dm(
    pool: &SqlitePool,
    keys: &Keys,
    db_event: DbEvent,
    relay_url: &Url,
) -> Result<DatabaseSuccessEventKind, Error> {
    // Convert DbEvent to DbMessage
    let db_message = DbMessage::from_db_event(db_event, Some(relay_url.to_owned()))?;
    tracing::info!("Inserting external message");

    // Insert message into the database and get the message ID
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    let db_message = db_message.with_id(msg_id);

    // Decrypt the message content
    let content = db_message.decrypt_message(keys)?;

    // Determine if the message is from the user or received from another user
    let (contact_pubkey, is_from_user) = if db_message.im_author(&keys.public_key()) {
        (db_message.to_pubkey(), true)
    } else {
        (db_message.from_pubkey(), false)
    };

    // Fetch the associated contact from the database
    match DbContact::fetch_one(pool, &contact_pubkey).await? {
        Some(mut db_contact) => {
            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;
            Ok(DatabaseSuccessEventKind::ReceivedDM((
                db_contact,
                chat_message,
            )))
        }
        None => {
            // Create a new contact and insert it into the database
            let mut db_contact = DbContact::new(&contact_pubkey);
            insert_contact(keys, pool, &db_contact).await?;

            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;

            Ok(DatabaseSuccessEventKind::NewDMAndContact((
                db_contact,
                chat_message,
            )))
        }
    }
}

// async fn update_unseen_count(
//     pool: &SqlitePool,
//     keys: &Keys,
//     mut db_contact: DbContact,
// ) -> Result<DbContact, Error> {
//     let own_pubkey = keys.public_key();
//     let chat = DbChat::new(&own_pubkey, db_contact.pubkey());
//     // sera q é necessário fazer isso
//     // ou somente falar unseen_messages += 1?
//     let count = chat.fetch_unseen_count(pool).await?;
//     DbContact::update_unseen_count(pool, &mut db_contact, count).await?;

//     Ok(db_contact)
// }

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

async fn insert_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<DbContact, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactInsert);
    }
    DbContact::insert(pool, &db_contact).await?;
    Ok(db_contact.to_owned())
}

async fn insert_batch_of_contacts(
    keys: &Keys,
    pool: &SqlitePool,
    db_contacts: &[DbContact],
) -> Result<(), Error> {
    for db_contact in db_contacts {
        if let Err(e) = insert_contact(keys, pool, db_contact).await {
            tracing::error!("{}", e);
        }
    }
    Ok(())
}

const IN_MEMORY: bool = false;
