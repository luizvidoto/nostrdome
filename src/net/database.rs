use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};

use crate::db::{Database, DbRelay};
use crate::types::RelayUrl;

#[derive(Debug, Clone)]
pub struct DbConnection(mpsc::UnboundedSender<Message>);
impl DbConnection {
    pub fn send(&mut self, message: Message) -> Result<(), String> {
        self.0.unbounded_send(message).map_err(|e| e.to_string())
    }
}
pub enum State {
    Disconnected,
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
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected(DbConnection),
    Disconnected,
    Error(String),
    DatabaseSuccessEvent(DatabaseSuccessEventKind),
    GotDbRelays(Vec<DbRelay>),
}
#[derive(Debug, Clone)]
pub enum Message {
    FetchRelays,
    AddRelay(DbRelay),
    UpdateRelay(DbRelay),
    DeleteRelay(RelayUrl),
}

pub fn database_connect() -> Subscription<Event> {
    struct DBConnect;
    let id = std::any::TypeId::of::<DBConnect>();

    subscription::unfold(id, State::Disconnected, |state| async move {
        match state {
            State::Disconnected => match Database::new(true).await {
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
                        Some(Event::Connected(DbConnection(sender))),
                        State::Connected { database, receiver },
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to init database");
                    tracing::error!("{}", e);
                    tracing::warn!("Trying again in 2 secs");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    (Some(Event::Disconnected), State::Disconnected)
                }
            },
            State::Connected {
                database,
                mut receiver,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::FetchRelays => {
                                match DbRelay::fetch(&database.pool, None).await {
                                    Ok(relays) => {
                                        (Some(Event::GotDbRelays(relays)), State::Connected {database,receiver})
                                    },
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::DeleteRelay(relay_url) => {
                                match DbRelay::delete(&database.pool, relay_url).await {
                                    Ok(_) => {
                                        (Some(Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayDeleted)), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::AddRelay(db_relay) => {
                                match DbRelay::insert(&database.pool, db_relay).await {
                                    Ok(_) => {
                                        (Some(Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayCreated)), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::Connected {database,receiver})
                                    }
                                }
                            }
                            Message::UpdateRelay(db_relay) => {
                                match DbRelay::update(&database.pool, db_relay).await {
                                    Ok(_) => {
                                        (Some(Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated)), State::Connected {database,receiver})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::Connected {database,receiver})
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
