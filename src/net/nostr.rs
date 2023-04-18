use async_stream::stream;
use iced::futures::{channel::mpsc, stream::Fuse, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    Client, EventId, Filter, Keys, Kind, Relay, RelayMessage, RelayPoolNotification, Timestamp,
};
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NostrConnection(mpsc::UnboundedSender<Message>);
impl NostrConnection {
    pub fn send(&mut self, message: Message) -> Result<(), String> {
        self.0.unbounded_send(message).map_err(|e| e.to_string())
    }
}
pub enum State {
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

#[derive(Debug, Clone)]
pub enum Event {
    Connected(NostrConnection),
    RelaysConnected,
    Disconnected,
    Error(String),
    GotRelays(Vec<Relay>),
    GotOwnEvents(Vec<nostr_sdk::Event>),
    GotPublicKey(XOnlyPublicKey),
    SomeEventSuccessId(EventId),
    GotEvent(String),
    RelayRemoved(String),
    NostrEvent(nostr_sdk::Event),
    RelayMessage(RelayMessage),
    Shutdown,
}
#[derive(Debug, Clone)]
pub enum Message {
    ConnectRelays,
    SendDMTo((XOnlyPublicKey, String)),
    ShowPublicKey,
    ShowRelays,
    ListOwnEvents,
    GetEventById(String),
    RemoveRelay(String),
}

// acc1
// secret  "4510459b74db68371be462f19ef4f7ef1e6c5a95b1d83a7adf00987c51ac56fe"
// hex_pub "8860df7d3b24bfb40fe5bdd2041663d35d3e524ce7376628aa55a7d3e624ac46"

// acc2
// secret  "nsec14q6klhw6u83y2k3l5ey2pf609wl60qnmpt7yu3fak3d6p3u26nwqd0jhvl"
// hex_pub "4cced2fb18ff00d32b4f01a65e9cfcc7e3c607024fd8c6604186de40e50aae03"

// acc3
// secret  "nsec1x2aq7r90upmen8f860d64gxr9evvtt6cl8w9uvdc5dnte36gjlsqkkvcdf"
// hex_pub "9e45b5e573adfb70be9f81e6f19e3df334fa24b3a7273859104d399ccbf64e94"

pub fn nostr_connect(keys: Keys) -> Subscription<Event> {
    struct NostrClient;
    let id = std::any::TypeId::of::<NostrClient>();

    subscription::unfold(id, State::Disconnected { keys }, |state| async move {
        match state {
            State::Disconnected { keys } => {
                // Create new client
                let nostr_client = Client::new(&keys);
                let recv_msgs_sub = Filter::new()
                    .pubkey(keys.public_key())
                    .kind(Kind::EncryptedDirectMessage)
                    .since(Timestamp::now());

                let sent_msgs_sub = Filter::new()
                    .author(keys.public_key().to_string())
                    .kind(Kind::EncryptedDirectMessage)
                    .since(Timestamp::now());

                nostr_client
                    .subscribe(vec![recv_msgs_sub, sent_msgs_sub])
                    .await;

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
                    Event::Connected(NostrConnection(sender)),
                    State::Connected {
                        receiver,
                        nostr_client,
                        keys,
                        notifications_stream,
                    },
                )
            }
            State::Connected {
                mut receiver,
                nostr_client,
                keys,
                mut notifications_stream,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::ConnectRelays => {
                                match nostr_client.add_relay("ws://192.168.15.151:8080", None).await {
                                    Ok(_) => {
                                        nostr_client.connect().await;
                                        (Event::RelaysConnected, State::Connected {receiver, nostr_client, keys, notifications_stream})
                                    },
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {receiver, nostr_client, keys, notifications_stream})
                                    }
                                }
                            }
                            Message::SendDMTo((pubkey, msg)) => {
                                match nostr_client.send_direct_msg(pubkey, msg).await {
                                    Ok(ev_id)=> {
                                        (Event::SomeEventSuccessId(ev_id), State::Connected {receiver, nostr_client, keys, notifications_stream})
                                    }
                                    Err(e) => {
                                        (Event::Error(e.to_string()), State::Connected {receiver, nostr_client, keys, notifications_stream})
                                    }
                                }
                            }
                            Message::ShowPublicKey => {
                                let pb_key = keys.public_key();
                                (Event::GotPublicKey(pb_key), State::Connected {receiver, nostr_client, keys, notifications_stream})
                            }
                            Message::GetEventById(_ev_id) => {
                                (Event::GotEvent(_ev_id), State::Connected {receiver, nostr_client, keys, notifications_stream})
                            }
                            Message::ShowRelays => {
                                let relays = nostr_client.relays().await;
                                let relays:Vec<_> = relays.into_iter().map(|r| r.1).collect();
                                (
                                    Event::GotRelays(relays),
                                    State::Connected {

                                        receiver,
                                        nostr_client,
                                        keys,
                                        notifications_stream
                                    },
                                )
                            },
                            Message::RemoveRelay(id) => {
                                tracing::info!("remove_relay: {}", id);
                                (Event::RelayRemoved(id), State::Connected {receiver, nostr_client, keys, notifications_stream})
                            },
                            Message::ListOwnEvents => {
                                let recv_msgs_sub = Filter::new()
                                    .pubkey(keys.public_key())
                                    .kind(Kind::EncryptedDirectMessage)
                                    .until(Timestamp::now());

                                let sent_msgs_sub = Filter::new()
                                    .author(keys.public_key().to_string())
                                    .kind(Kind::EncryptedDirectMessage)
                                    .until(Timestamp::now());

                                let timeout = Duration::from_secs(10);
                                match nostr_client.get_events_of(vec![recv_msgs_sub, sent_msgs_sub], Some(timeout)).await {
                                    Ok(events) => (Event::GotOwnEvents(events), State::Connected {receiver, nostr_client, keys, notifications_stream}),
                                    Err(e) => (Event::Error(e.to_string()), State::Connected {receiver, nostr_client, keys, notifications_stream}),
                                }
                            }
                        }
                    }
                    notification = notifications_stream.select_next_some() => {
                        match notification {
                            RelayPoolNotification::Event(_url, event) => {
                                // tracing::info!("url: {}", _url);
                                // tracing::info!("event: {:?}", event);
                                (Event::NostrEvent(event), State::Connected {receiver, nostr_client, keys, notifications_stream})
                            },
                            RelayPoolNotification::Message(_url, relay_msg) => {
                                (Event::RelayMessage(relay_msg), State::Connected {receiver, nostr_client, keys, notifications_stream})
                            }
                            RelayPoolNotification::Shutdown => {
                                (Event::Shutdown, State::Connected {receiver, nostr_client, keys, notifications_stream})
                            }
                        }
                    },
                }
            }
        }
    })
}
