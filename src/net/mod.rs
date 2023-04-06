mod list_events;
// mod new;

use async_stream::stream;
use iced::futures::{channel::mpsc, stream::Fuse, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::prelude::*;
use std::pin::Pin;
use std::time::Duration;

use crate::db::{Database, DbRelay};
use crate::types::RelayUrl;

#[derive(Debug, Clone)]
pub struct Connection(mpsc::UnboundedSender<Message>);
impl Connection {
    pub fn send(&mut self, message: Message) -> Result<(), String> {
        // Err(String::from("Error sending to nostr_client"))
        // self.0.try_send(message).map_err(|e| e.to_string())
        self.0.unbounded_send(message).map_err(|e| e.to_string())
    }
}
pub enum State {
    Disconnected,
    ConnectedDB {
        database: Database,
    },
    ConnectedAll {
        database: Database,
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Client,
        my_keys: Keys,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
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
    Connected(Connection),
    Disconnected,
    Error(String),
    GotRelays(Vec<Relay>),
    GotOwnEvents(Vec<nostr::Event>),
    DirectMessage(String),
    NostrEvent(nostr_sdk::Event),
    GotPublicKey(XOnlyPublicKey),
    SomeEventSuccessId(EventId),
    DatabaseSuccessEvent(DatabaseSuccessEventKind),
    GotDbRelays(Vec<DbRelay>),
}
#[derive(Debug, Clone)]
pub enum Message {
    SendDMTo((XOnlyPublicKey, String)),
    ShowPublicKey,
    ShowRelays,
    AddRelay(String),
    RemoveRelay(usize),
    ListOwnEvents,
    GetEventById(String),
    FetchRelays,
    UpdateRelay(DbRelay),
    DeleteRelay(RelayUrl),
}
// acc1
// secret  "4510459b74db68371be462f19ef4f7ef1e6c5a95b1d83a7adf00987c51ac56fe"

// acc2
// secret  "nsec14q6klhw6u83y2k3l5ey2pf609wl60qnmpt7yu3fak3d6p3u26nwqd0jhvl"
// bech_32 "npub1fn8d97ccluqdx260qxn9a88ucl3uvpczflvvvczpsm0ypeg24cps4n52a5"

// acc3
// secret  "nsec1x2aq7r90upmen8f860d64gxr9evvtt6cl8w9uvdc5dnte36gjlsqkkvcdf"
// bech_32 "npub1nezmtetn4hahp05ls8n0r83a7v605f9n5unnskgsf5ueejlkf62qyhlsam"

const PRIVATE_KEY: &'static str = "nsec14q6klhw6u83y2k3l5ey2pf609wl60qnmpt7yu3fak3d6p3u26nwqd0jhvl";

pub fn nostr_connect() -> Subscription<Event> {
    struct Connect;
    let id = std::any::TypeId::of::<Connect>();

    subscription::unfold(id, State::Disconnected, move |state| async move {
        match state {
            State::Disconnected => match Database::new(true).await {
                Ok(database) => (None, State::ConnectedDB { database }),
                Err(e) => {
                    tracing::error!("Failed to init database");
                    tracing::error!("{:?}", e);
                    tracing::warn!("Trying again in 2 secs");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    (Some(Event::Disconnected), State::Disconnected)
                }
            },
            State::ConnectedDB { database } => {
                let my_keys = Keys::from_sk_str(PRIVATE_KEY).unwrap();

                // Create new client
                let nostr_client = Client::new(&my_keys);

                // Add relays to database
                for r in vec![
                    "wss://eden.nostr.land",
                    "wss://relay.snort.social",
                    "wss://relay.nostr.band",
                    // "wss://nostr.fmt.wiz.biz",
                    // "wss://relay.damus.io",
                    // "wss://nostr.anchel.nl/",
                    // "ws://192.168.15.119:8080"
                ] {
                    if let Ok(url) = RelayUrl::try_from_str(r) {
                        let db_relay = DbRelay::new(url);
                        if let Err(e) = DbRelay::insert(&database.pool, db_relay).await {
                            tracing::error!("{}", e);
                        }
                    }

                    // if let Err(e) = nostr_client.add_relay(r, None).await {
                    //     println!("Error adding relay: {}", r);
                    //     println!("{:?}", e);
                    // }
                }

                // fetch relays
                let relays = match DbRelay::fetch(&database.pool, None).await {
                    Ok(relays) => relays,
                    Err(e) => {
                        tracing::error!("{}", e);
                        vec![]
                    }
                };

                // add relays to client
                for r in relays {
                    if let Err(e) = nostr_client.add_relay(r.url.0, None).await {
                        tracing::error!("{}", e);
                    }
                }

                // Connect to relays
                nostr_client.connect().await;

                let subscription = Filter::new()
                    .author(my_keys.public_key())
                    .pubkey(my_keys.public_key())
                    .since(Timestamp::now());

                nostr_client.subscribe(vec![subscription]).await;

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
                    Some(Event::Connected(Connection(sender))),
                    State::ConnectedAll {
                        database,
                        receiver,
                        nostr_client,
                        my_keys,
                        notifications_stream,
                    },
                )
            }
            State::ConnectedAll {
                database,
                mut receiver,
                nostr_client,
                my_keys,
                mut notifications_stream,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::DeleteRelay(relay_url) => {
                                match DbRelay::delete(&database.pool, relay_url).await {
                                    Ok(_) => {
                                        (Some(Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayDeleted)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                }
                            }
                            Message::UpdateRelay(db_relay) => {
                                match DbRelay::update(&database.pool, db_relay).await {
                                    Ok(_) => {
                                        (Some(Event::DatabaseSuccessEvent(DatabaseSuccessEventKind::RelayUpdated)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                }
                            }
                            Message::FetchRelays => {
                                match DbRelay::fetch(&database.pool, None).await {
                                    Ok(relays) => {
                                        (Some(Event::GotDbRelays(relays)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    },
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                }
                            }
                            Message::SendDMTo((pub_key, msg)) => {
                                match nostr_client.send_direct_msg(pub_key, msg).await {
                                    Ok(ev_id)=> {
                                        (Some(Event::SomeEventSuccessId(ev_id)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                    Err(e) => {
                                        (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                    }
                                }
                            }
                            Message::ShowPublicKey => {
                                let pb_key = my_keys.public_key();
                                (Some(Event::GotPublicKey(pb_key)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                            }
                            Message::GetEventById(_ev_id) => {
                                (None, State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                            }
                            Message::ShowRelays => {
                                let relays = nostr_client.relays().await;
                                let relays:Vec<_> = relays.into_iter().map(|r| r.1).collect();
                                (
                                    Some(Event::GotRelays(relays)),
                                    State::ConnectedAll {
                                        database,
                                        receiver,
                                        nostr_client,
                                        my_keys,
                                        notifications_stream
                                    },
                                )
                            },
                            Message::AddRelay(address) => {
                                match nostr_client.add_relay(&address, None).await {
                                    Ok(_) => {
                                        tracing::warn!("added_relay: {}", address);
                                        (
                                            None,
                                            State::ConnectedAll {
                                                database,
                                                receiver,
                                                nostr_client,
                                                my_keys,
                                                notifications_stream
                                            },
                                        )
                                    }
                                    Err(e) => (
                                        Some(Event::Error(e.to_string())),
                                        State::ConnectedAll {
                                            database,
                                            receiver,
                                            nostr_client,
                                            my_keys,
                                            notifications_stream
                                        },
                                    ),
                                }
                            },
                            Message::RemoveRelay(id) => {
                                tracing::warn!("remove_relay: {}", id);
                                (None, State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                            },
                            Message::ListOwnEvents => {
                                let sub = Filter::new()
                                    .author(my_keys.public_key())
                                    .since(Timestamp::from(1678050565))
                                    .until(Timestamp::now());
                                let timeout = Duration::from_secs(10);
                                match nostr_client.get_events_of(vec![sub], Some(timeout)).await {
                                    Ok(events) => (Some(Event::GotOwnEvents(events)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream}),
                                    Err(e) => (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream}),
                                }
                            }
                        }
                    }
                    notification = notifications_stream.select_next_some() => {
                        if let RelayPoolNotification::Event(_url, event) = notification {
                            tracing::warn!("url: {}", _url);
                            tracing::warn!("event: {:?}", event);
                            if event.kind == Kind::EncryptedDirectMessage {
                                let secret_key = match my_keys.secret_key() {
                                    Ok(sk) => sk,
                                    Err(e) => return (Some(Event::Error(e.to_string())), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream}),
                                };
                                match decrypt(&secret_key, &event.pubkey, &event.content) {
                                    Ok(msg) => return (Some(Event::DirectMessage(msg)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream}),
                                    Err(e) => return (Some(Event::Error(format!("Impossible to decrypt message: {}", e.to_string()))), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                                }
                            } else {
                                return (Some(Event::NostrEvent(event)), State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                            }
                        }
                        (None, State::ConnectedAll {database,receiver, nostr_client, my_keys, notifications_stream})
                    },
                }
            }
        }
    })
}

// client.connect().await;

// let subscription = Filter::new()
//     .pubkey(my_keys.public_key())
//     .since(Timestamp::now());

// client.subscribe(vec![subscription]).await;

// let mut notifications = client.notifications();
// while let Ok(notification) = notifications.recv().await {
//     if let RelayPoolNotification::Event(_url, event) = notification {
//         if event.kind == Kind::EncryptedDirectMessage {
//             if let Ok(msg) = decrypt(&my_keys.secret_key()?, &event.pubkey, &event.content) {
//                 println!("New DM: {}", msg);
//             } else {
//                 log::error!("Impossible to decrypt direct message");
//             }
//         } else {
//             println!("{:?}", event);
//         }
//     }
// }
