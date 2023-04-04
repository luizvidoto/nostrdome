use std::time::Duration;

use futures::channel::mpsc;
use futures::stream::StreamExt;
use iced::Subscription;
use iced_native::subscription;
// use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use nostr_sdk::prelude::*;

#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<Message>);
impl Connection {
    pub fn send(&mut self, message: Message) -> Result<(), String> {
        // Err(String::from("Error sending to nostr_client"))
        self.0.try_send(message).map_err(|e| e.to_string())
        // .expect("Error sending message to database");
    }
}
#[derive(Debug)]
pub enum State {
    Disconnected,
    Connected {
        receiver: mpsc::Receiver<Message>,
        nostr_client: Client,
        my_keys: Keys,
    },
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected(Connection),
    Disconnected,
    Error(String),
    GotRelays(Vec<Relay>),
    GotOwnEvents(Vec<nostr::Event>),
}
#[derive(Debug, Clone)]
pub enum Message {
    ShowRelays,
    AddRelay(String),
    RemoveRelay(usize),
    GetOwnEvents,
}

const PRIVATE_KEY: &'static str =
    "4510459b74db68371be462f19ef4f7ef1e6c5a95b1d83a7adf00987c51ac56fe";

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
                let (sender, receiver) = mpsc::channel(10);
                (
                    Some(Event::Connected(Connection(sender))),
                    State::Connected {
                        receiver,
                        nostr_client,
                        my_keys,
                    },
                )
            }
            State::Connected {
                mut receiver,
                nostr_client,
                my_keys,
            } => {
                futures::select! {
                    message = receiver.select_next_some() => {
                        match message {
                            Message::ShowRelays => {
                                let relays = nostr_client.relays().await;
                                let relays:Vec<_> = relays.into_iter().map(|r| r.1).collect();
                                (
                                    Some(Event::GotRelays(relays)),
                                    State::Connected {
                                        receiver,
                                        nostr_client,
                                        my_keys
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
                                                my_keys
                                            },
                                        )
                                    }
                                    Err(e) => (
                                        Some(Event::Error(e.to_string())),
                                        State::Connected {
                                            receiver,
                                            nostr_client,
                                            my_keys
                                        },
                                    ),
                                }
                            },
                            Message::RemoveRelay(id) => {
                                println!("remove_relay: {}", id);
                                (None, State::Connected {receiver, nostr_client, my_keys})
                            },
                            Message::GetOwnEvents => {
                                    let sub = Filter::new()
                                        .pubkeys(vec![my_keys.public_key()])
                                        .since(Timestamp::from(1675555023));
                                    let timeout = Duration::from_secs(10);
                                    match nostr_client.get_events_of(vec![sub], Some(timeout)).await {
                                        Ok(events) => (Some(Event::GotOwnEvents(events)), State::Connected {receiver, nostr_client, my_keys}),
                                        Err(e) => (Some(Event::Error(e.to_string())), State::Connected {receiver, nostr_client, my_keys}),
                                    }
                            }
                        }
                    }
                }
            }
        }
    })
}
