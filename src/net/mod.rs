// pub mod database;
// mod list_events;
// mod new;
// pub mod nostr;

use crate::db::{Database, DbContact, DbEvent, DbRelay};
use crate::error::Error;
use crate::types::{ChatMessage, RelayUrl};
use async_stream::stream;
use futures::channel::mpsc::TrySendError;
use futures::Future;
use iced::futures::stream::Fuse;
use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::prelude::decrypt;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    Client, EventId, Filter, Keys, Kind, Relay, RelayMessage, RelayOptions, RelayPoolNotification,
    Timestamp,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
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
    Connected(BackEndConnection),
    Disconnected,
    Error(String),
    // NOSTR CLIENT EVENTS
    RelaysConnected,
    GotRelays(Vec<(Relay, DbRelay)>),
    GotOwnEvents(Vec<nostr_sdk::Event>),
    GotPublicKey(XOnlyPublicKey),
    SomeEventSuccessId(EventId),
    GotEvent(String),
    RelayRemoved(String),
    NostrEvent(nostr_sdk::Event),
    RelayMessage(RelayMessage),
    Shutdown,
    RelayConnected(RelayUrl),

    // DATABASE EVENTS
    DatabaseSuccessEvent(DatabaseSuccessEventKind),
    // GotDbRelays(Vec<DbRelay>),
    GotMessages(Vec<ChatMessage>),
    GotContacts(Vec<DbContact>),
    None,
}
#[derive(Debug, Clone)]
pub enum Message {
    // NOSTR CLIENT MESSAGES
    ConnectRelays,
    ConnectToRelay(RelayUrl),
    SendDMTo((XOnlyPublicKey, String)),
    ShowPublicKey,
    ListOwnEvents,

    // DATABASE MESSAGES
    FetchMessages(XOnlyPublicKey),
    AddContact(DbContact),
    FetchContacts,
    UpdateContact(DbContact),
    DeleteContact(XOnlyPublicKey),
    InsertEvent(nostr_sdk::Event),
    FetchRelays,
    AddRelay(DbRelay),
    UpdateRelay(DbRelay),
    DeleteRelay(RelayUrl),
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
                    let database = match Database::new(in_memory, &keys.public_key().to_string())
                        .await
                    {
                        Ok(database) => {
                            // Add relays to database
                            for r in vec![
                                "wss://eden.nostr.land",
                                "wss://relay.snort.social",
                                // "wss://relay.nostr.band",
                                // "wss://nostr.fmt.wiz.biz",
                                // "wss://relay.damus.io",
                                // "wss://nostr.anchel.nl/",
                                // "ws://192.168.15.119:8080"
                                "ws://192.168.15.151:8080",
                            ] {
                                if let Ok(url) = RelayUrl::try_from_str(r) {
                                    let db_relay = DbRelay::new(url);
                                    if let Err(e) = DbRelay::insert(&database.pool, &db_relay).await
                                    {
                                        tracing::error!("{}", e);
                                    }
                                    println!("Inserted relay: {}", &db_relay.url.as_str());
                                }
                            }

                            database
                        }
                        Err(e) => {
                            tracing::error!("Failed to init database");
                            tracing::error!("{}", e);
                            tracing::warn!("Trying again in 2 secs");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            return (Event::Disconnected, State::Disconnected { keys, in_memory });
                        }
                    };

                    // Create new client
                    let nostr_client = Client::new(&keys);
                    let recv_msgs_sub = Filter::new()
                        .pubkey(keys.public_key())
                        .kind(Kind::EncryptedDirectMessage)
                        .since(Timestamp::now());

                    // let sent_msgs_sub = Filter::new()
                    //     .author(keys.public_key().to_string())
                    //     .kind(Kind::EncryptedDirectMessage)
                    //     .since(Timestamp::now());

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
                                Message::FetchRelays => {
                                    process_async_fn(
                                        fetch_relays(&database.pool, &nostr_client),
                                        |relays| Event::GotRelays(relays),
                                    )
                                    .await
                                }
                                Message::ConnectToRelay(relay_url) => {
                                    process_async_fn(
                                        nostr_client.connect_relay(relay_url.as_str()),
                                        |_| Event::RelayConnected(relay_url.clone())
                                    ).await
                                }
                                Message::DeleteRelay(relay_url) => {
                                    process_async_fn(
                                        DbRelay::delete(&database.pool, &relay_url),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayDeleted),
                                    )
                                    .await
                                }
                                Message::AddRelay(db_relay) => {
                                    process_async_fn(
                                        DbRelay::insert(&database.pool, &db_relay),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayCreated),
                                    )
                                    .await
                                }
                                Message::UpdateRelay(db_relay) => {
                                    process_async_fn(
                                        update_relay_db_and_client(&database.pool, &nostr_client, &db_relay),
                                        |_| Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated),
                                    )
                                    .await
                                }
                                Message::ConnectRelays => {
                                    process_async_fn(
                                        connect_relays(&database.pool, &nostr_client),
                                        |_| Event::RelaysConnected,
                                    )
                                    .await
                                }
                                Message::SendDMTo((pubkey, msg)) => {
                                    process_async_fn(
                                        nostr_client.send_direct_msg(pubkey, msg),
                                        |event_id| Event::SomeEventSuccessId(event_id),
                                    )
                                    .await
                                }
                                Message::ShowPublicKey => {
                                    let pb_key = keys.public_key();
                                    Event::GotPublicKey(pb_key)
                                }
                                Message::ListOwnEvents => {
                                    let recv_msgs_sub = Filter::new()
                                        .pubkey(keys.public_key())
                                        .kind(Kind::EncryptedDirectMessage)
                                        .until(Timestamp::now());
                                    let timeout = Duration::from_secs(10);
                                    process_async_fn(
                                        nostr_client.get_events_of(vec![recv_msgs_sub], Some(timeout)),
                                        |events| Event::GotOwnEvents(events),

                                    )
                                    .await
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

async fn fetch_relays(
    pool: &SqlitePool,
    nostr_client: &Client,
) -> Result<Vec<(Relay, DbRelay)>, Error> {
    let db_relays = DbRelay::fetch(pool, None).await?;
    let relays = nostr_client.relays().await;
    let mut new_map: HashMap<RelayUrl, (Relay, DbRelay)> = HashMap::new();
    let relays: HashMap<RelayUrl, Relay> = relays
        .into_iter()
        .filter_map(|(url, relay)| {
            if let Ok(relay_url) = RelayUrl::try_from_str(url.as_str()) {
                Some((relay_url, relay))
            } else {
                None
            }
        })
        .collect();
    for db_r in db_relays {
        if let Some(relay) = relays.get(&db_r.url) {
            new_map.insert(db_r.url.clone(), (relay.clone(), db_r));
        }
    }

    Ok(new_map
        .into_iter()
        .map(|(_url, (r, db_r))| (r, db_r))
        .collect())
}

async fn update_relay_db_and_client(
    pool: &SqlitePool,
    nostr_client: &Client,
    db_relay: &DbRelay,
) -> Result<(), Error> {
    // this doesn't work
    // it disconnects every time a little change in the db_happens
    nostr_client.remove_relay(db_relay.url.as_str()).await?;
    nostr_client
        .add_relay_with_opts(
            db_relay.url.as_str(),
            None,
            RelayOptions::new(db_relay.read, db_relay.write),
        )
        .await?;
    nostr_client.connect_relay(db_relay.url.as_str()).await?;
    DbRelay::update(pool, db_relay).await?;
    Ok(())
}

async fn connect_relays(pool: &SqlitePool, nostr_client: &Client) -> Result<(), Error> {
    let relays = DbRelay::fetch(pool, None).await?;
    for r in relays {
        nostr_client
            .add_relay_with_opts(r.url.as_str(), None, RelayOptions::new(true, true))
            .await?;
    }
    nostr_client.connect();
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
