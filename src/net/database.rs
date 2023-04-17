use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::prelude::decrypt;
use nostr_sdk::{Keys, Kind};

use crate::db::{Database, DbEvent, DbRelay};
use crate::types::{ChatMessage, RelayUrl};

#[derive(Debug, Clone)]
pub struct DbConnection(mpsc::UnboundedSender<Message>);
impl DbConnection {
    pub fn send(&mut self, message: Message) -> Result<(), String> {
        self.0.unbounded_send(message).map_err(|e| e.to_string())
    }
}
pub enum State {
    Disconnected(bool),
    Connected {
        database: Database,
        receiver: mpsc::UnboundedReceiver<Message>,
    },
}

#[derive(Debug, Clone)]
pub enum DatabaseSuccessEventKind {
    RelayCreated,
    RelayUpdated,
    RelayDeleted,
    EventInserted(nostr_sdk::Event),
    NewDM(ChatMessage),
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected(DbConnection),
    Disconnected,
    Error(String),
    DatabaseSuccessEvent(DatabaseSuccessEventKind),
    GotDbRelays(Vec<DbRelay>),
    GotMessages(Vec<ChatMessage>),
    GotNewMessage(ChatMessage),
    None,
}
#[derive(Debug, Clone)]
pub enum Message {
    FetchMessages(Keys),
    InsertEvent { keys: Keys, event: nostr_sdk::Event },
    FetchRelays,
    AddRelay(DbRelay),
    UpdateRelay(DbRelay),
    DeleteRelay(RelayUrl),
}

pub fn database_connect(in_memory: bool) -> Subscription<Event> {
    struct DBConnect;
    let id = std::any::TypeId::of::<DBConnect>();

    subscription::unfold(id, State::Disconnected(in_memory), |state| async move {
        match state {
            State::Disconnected(in_memory) => match Database::new(in_memory).await {
                Ok(database) => {
                    let (sender, receiver) = mpsc::unbounded();
                    // Add relays to database
                    for r in vec![
                        // "wss://eden.nostr.land",
                        // "wss://relay.snort.social",
                        // "wss://relay.nostr.band",
                        // "wss://nostr.fmt.wiz.biz",
                        // "wss://relay.damus.io",
                        // "wss://nostr.anchel.nl/",
                        // "ws://192.168.15.119:8080"
                        "ws://192.168.15.151:8080",
                    ] {
                        if let Ok(url) = RelayUrl::try_from_str(r) {
                            let db_relay = DbRelay::new(url);
                            if let Err(e) = DbRelay::insert(&database.pool, db_relay).await {
                                tracing::error!("{}", e);
                            }
                        }
                    }
                    (
                        Event::Connected(DbConnection(sender)),
                        State::Connected { database, receiver },
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to init database");
                    tracing::error!("{}", e);
                    tracing::warn!("Trying again in 2 secs");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    (Event::Disconnected, State::Disconnected(in_memory))
                }
            },
            State::Connected {
                database,
                mut receiver,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::FetchMessages(keys) => {
                                match DbEvent::fetch(
                                    &database.pool,
                                    Some(&format!("pubkey = '{}'", &keys.public_key())),
                                )
                                .await
                                {
                                    Ok(events) => {
                                        let secret_key = match keys.secret_key() {
                                            Ok(sk) => sk,
                                            Err(e) => return (Event::Error(e.to_string()), State::Connected {database, receiver}),
                                        };
                                        let messages:Vec<_> = events
                                            .iter()
                                            .filter_map(|ev| {
                                                if let Kind::EncryptedDirectMessage = ev.kind {
                                                    match decrypt(&secret_key, &ev.pubkey, &ev.content) {
                                                        Ok(msg) => Some(ChatMessage::from_event(ev, msg)),
                                                        Err(_e) => None,
                                                    }
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                            (Event::GotMessages(messages), State::Connected {database,receiver})
                                    }
                                    Err(e) => (Event::Error(e.to_string()), State::Connected {database,receiver}),
                                }
                            },
                            Message::InsertEvent {keys,event} => {
                                match DbEvent::insert(&database.pool, DbEvent::from(&event)).await {
                                    Ok(rows_changed) => {
                                        if rows_changed > 0 {
                                            if let Kind::EncryptedDirectMessage = event.kind {
                                                let secret_key = match keys.secret_key() {
                                                    Ok(sk) => sk,
                                                    Err(e) => return (Event::Error(e.to_string()), State::Connected {database, receiver}),
                                                };
                                                match decrypt(&secret_key, &event.pubkey, &event.content) {
                                                    Ok(msg) => (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::NewDM(ChatMessage::from_event(&event, msg))), State::Connected {database,receiver}),
                                                    Err(_e) => (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::NewDM(ChatMessage::from_event(&event, &event.content))), State::Connected {database,receiver}),
                                                    // Err(e) => (Event::Error(e.to_string()), State::Connected {database,receiver}),
                                                }
                                            } else {
                                                (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::EventInserted(event)), State::Connected {database,receiver})
                                            }
                                        } else {
                                            (Event::None, State::Connected {database,receiver})
                                        }
                                    },
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::FetchRelays => {
                                match DbRelay::fetch(&database.pool, None).await {
                                    Ok(relays) => {
                                        (Event::GotDbRelays(relays), State::Connected {database,receiver})
                                    },
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::DeleteRelay(relay_url) => {
                                match DbRelay::delete(&database.pool, relay_url).await {
                                    Ok(_) => {
                                        (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayDeleted), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::AddRelay(db_relay) => {
                                match DbRelay::insert(&database.pool, db_relay).await {
                                    Ok(_) => {
                                        (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayCreated), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::UpdateRelay(db_relay) => {
                                match DbRelay::update(&database.pool, db_relay).await {
                                    Ok(_) => {
                                        (Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {database,receiver})
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
