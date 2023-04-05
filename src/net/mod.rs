mod list_events;
mod new;

use async_stream::stream;
use iced::futures::{channel::mpsc, stream::Fuse, StreamExt};
use iced::{subscription, Subscription};
use nostr_sdk::prelude::*;
use std::pin::Pin;
use std::time::Duration;

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
    Connected {
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Client,
        my_keys: Keys,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
    },
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
}
#[derive(Debug, Clone)]
pub enum Message {
    ShowPublicKey,
    ShowRelays,
    AddRelay(String),
    RemoveRelay(usize),
    ListOwnEvents,
    GetEventById(String),
}
// acc1
// "4510459b74db68371be462f19ef4f7ef1e6c5a95b1d83a7adf00987c51ac56fe"
// acc2
// "nsec14q6klhw6u83y2k3l5ey2pf609wl60qnmpt7yu3fak3d6p3u26nwqd0jhvl"
const PRIVATE_KEY: &'static str = "nsec14q6klhw6u83y2k3l5ey2pf609wl60qnmpt7yu3fak3d6p3u26nwqd0jhvl";

pub fn nostr_connect() -> Subscription<Event> {
    struct Connect;
    let id = std::any::TypeId::of::<Connect>();

    subscription::unfold(id, State::Disconnected, move |state| async move {
        match state {
            State::Disconnected => {
                let my_keys = Keys::from_sk_str(PRIVATE_KEY).unwrap();
                // Show bech32 public key
                let bech32_pubkey: String = my_keys.public_key().to_bech32().unwrap();
                println!("Bech32 PubKey: {}", bech32_pubkey);

                // Create new client
                let nostr_client = Client::new(&my_keys);
                // Add relays
                for r in vec![
                    "wss://eden.nostr.land",
                    "wss://relay.snort.social",
                    "wss://relay.nostr.band",
                    // "wss://nostr.fmt.wiz.biz",
                    // "wss://relay.damus.io",
                    // "wss://nostr.anchel.nl/",
                    // "ws://192.168.15.119:8080"
                ] {
                    if let Err(e) = nostr_client.add_relay(r, None).await {
                        println!("Error adding relay: {}", r);
                        println!("{:?}", e);
                    }
                }
                // Connect to relays
                nostr_client.connect().await;

                let subscription = Filter::new()
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
                    State::Connected {
                        receiver,
                        nostr_client,
                        my_keys,
                        notifications_stream,
                    },
                )
            }
            State::Connected {
                mut receiver,
                nostr_client,
                my_keys,
                mut notifications_stream,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::ShowPublicKey => {
                                let pb_key = my_keys.public_key();
                                (Some(Event::GotPublicKey(pb_key)), State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                            }
                            Message::GetEventById(_ev_id) => {
                                (None, State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                            }
                            Message::ShowRelays => {
                                let relays = nostr_client.relays().await;
                                let relays:Vec<_> = relays.into_iter().map(|r| r.1).collect();
                                (
                                    Some(Event::GotRelays(relays)),
                                    State::Connected {
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
                                        println!("added_relay: {}", address);
                                        (
                                            None,
                                            State::Connected {
                                                receiver,
                                                nostr_client,
                                                my_keys,
                                                notifications_stream
                                            },
                                        )
                                    }
                                    Err(e) => (
                                        Some(Event::Error(e.to_string())),
                                        State::Connected {
                                            receiver,
                                            nostr_client,
                                            my_keys,
                                            notifications_stream
                                        },
                                    ),
                                }
                            },
                            Message::RemoveRelay(id) => {
                                println!("remove_relay: {}", id);
                                (None, State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                            },
                            Message::ListOwnEvents => {
                                let sub = Filter::new()
                                    .author(my_keys.public_key())
                                    .since(Timestamp::from(1678050565))
                                    .until(Timestamp::now());
                                let timeout = Duration::from_secs(10);
                                match nostr_client.get_events_of(vec![sub], Some(timeout)).await {
                                    Ok(events) => (Some(Event::GotOwnEvents(events)), State::Connected {receiver, nostr_client, my_keys, notifications_stream}),
                                    Err(e) => (Some(Event::Error(e.to_string())), State::Connected {receiver, nostr_client, my_keys, notifications_stream}),
                                }
                            }
                        }
                    }
                    notification = notifications_stream.select_next_some() => {
                        if let RelayPoolNotification::Event(_url, event) = notification {
                            if event.kind == Kind::EncryptedDirectMessage {
                                let secret_key = match my_keys.secret_key() {
                                    Ok(sk) => sk,
                                    Err(e) => return (Some(Event::Error(e.to_string())), State::Connected {receiver, nostr_client, my_keys, notifications_stream}),
                                };
                                if let Ok(msg) = decrypt(&secret_key, &event.pubkey, &event.content) {
                                    return (Some(Event::DirectMessage(msg)), State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                                }else {
                                    return (Some(Event::Error("Impossible to decrypt message.".into())), State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                                }
                            } else {
                                return (Some(Event::NostrEvent(event)), State::Connected {receiver, nostr_client, my_keys, notifications_stream})
                            }
                        }
                        (None, State::Connected {receiver, nostr_client, my_keys, notifications_stream})
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
