use crate::db::{Database, DbEvent};
use crate::net::events::backend::{self, backend_processing};
use crate::net::events::frontend::Event;
use crate::net::events::nostr::{client_with_stream, NostrSdkWrapper};
use crate::net::logic::{delete_contact, import_contacts, insert_contact, update_contact};
use futures::channel::mpsc;
use futures::{Future, StreamExt};
use iced::futures::stream::Fuse;
use iced::Subscription;
use iced_native::subscription;
use logic::*;
use nostr_sdk::{Keys, RelayPoolNotification};
use std::pin::Pin;

mod back_channel;
pub(crate) mod events;
mod logic;
mod messages;

pub(crate) use self::back_channel::BackEndConnection;
use self::events::backend::BackEndInput;
use self::events::nostr::{NostrInput, NostrOutput};
pub(crate) use messages::Message;

pub enum State {
    Disconnected {
        in_memory: bool,
        keys: Keys,
    },
    Connected {
        in_memory: bool,
        keys: Keys,
        database: Option<Database>,
        receiver: mpsc::UnboundedReceiver<Message>,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
        backend_sender: mpsc::Sender<backend::BackEndInput>,
        backend_receiver: mpsc::Receiver<BackEndInput>,
        client_in_receiver: mpsc::Receiver<NostrInput>,
        client_in_sender: mpsc::Sender<NostrInput>,
        client_out_receiver: mpsc::Receiver<NostrOutput>,
        client_out_sender: mpsc::Sender<NostrOutput>,
        client_wrapper: NostrSdkWrapper,
    },
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<Event>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub = subscription::unfold(
        id,
        State::Disconnected {
            keys: keys.clone(),
            in_memory: IN_MEMORY,
        },
        |state| async move {
            match state {
                State::Disconnected { keys, in_memory } => {
                    let (sender, receiver) = mpsc::unbounded();
                    let (backend_sender, backend_receiver) = mpsc::channel(100);
                    let (client_in_sender, client_in_receiver) = mpsc::channel(100);
                    let (client_out_sender, client_out_receiver) = mpsc::channel(100);
                    let (client_wrapper, notifications_stream) = NostrSdkWrapper::new(&keys).await;

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
                        Event::Connected(BackEndConnection::new(sender)),
                        State::Connected {
                            in_memory,
                            database: Some(database),
                            notifications_stream,
                            receiver,
                            keys,
                            backend_receiver,
                            backend_sender,
                            client_in_receiver,
                            client_in_sender,
                            client_out_receiver,
                            client_out_sender,
                            client_wrapper,
                        },
                    )
                }
                State::Connected {
                    mut notifications_stream,
                    in_memory,
                    database,
                    mut receiver,
                    keys,
                    mut backend_sender,
                    mut backend_receiver,
                    mut client_in_receiver,
                    mut client_in_sender,
                    mut client_out_receiver,
                    mut client_out_sender,
                    client_wrapper,
                } => match database {
                    Some(database) => {
                        let event: Event = futures::select! {
                            nostr_input = client_in_receiver.select_next_some() => {
                                client_wrapper.process_in(
                                    &mut client_out_sender,
                                    &keys,
                                    nostr_input
                                ).await;
                                Event::None
                            }
                            nostr_output = client_out_receiver.select_next_some() => {
                                match nostr_output {
                                    NostrOutput::Ok(event) => event,
                                    NostrOutput::Error(e) => Event::Error(e.to_string()),
                                    NostrOutput::ToBackend(input) => to_backend_channel(&mut backend_sender, input),
                                }
                            }
                            backend_input = backend_receiver.select_next_some() => {
                                backend_processing(
                                    &database.pool,
                                    &keys,
                                    backend_input,
                                ).await
                            }
                            message = receiver.select_next_some() => {
                                match message {
                                    // -------- DATABASE MESSAGES -------
                                    Message::FetchRelayResponses(event_id) => {
                                        process_async_with_event(
                                            fetch_relays_responses(&database.pool, event_id),
                                        ).await
                                    }
                                    Message::AddToUnseenCount(db_contact) => {
                                        process_async_with_event(
                                            add_to_unseen_count(&database.pool, db_contact),
                                        ).await
                                    }
                                    Message::ImportContacts(db_contacts) => {
                                        process_async_with_event(
                                            import_contacts(&keys, &database.pool, &db_contacts),
                                        )
                                        .await
                                    }
                                    Message::AddContact(db_contact) => {
                                        process_async_with_event(
                                            insert_contact(&keys, &database.pool, &db_contact),
                                        )
                                        .await
                                    }
                                    Message::UpdateContact(db_contact) => {
                                        process_async_with_event(
                                            update_contact(&keys, &database.pool, &db_contact)
                                        )
                                        .await
                                    }
                                    Message::DeleteContact(contact) => {
                                        process_async_with_event(
                                            delete_contact(&database.pool, &contact),
                                        )
                                        .await
                                    }
                                    Message::FetchContacts => {
                                        process_async_with_event(
                                            fetch_contacts(&database.pool)
                                        )
                                        .await
                                    }
                                    Message::FetchRelays => {
                                        process_async_with_event(
                                            fetch_relays(&database.pool)
                                        )
                                        .await
                                    }
                                    Message::FetchMessages (contact) => {
                                        process_async_with_event(
                                            fetch_and_decrypt_chat(&keys, &database.pool, contact),
                                        )
                                        .await
                                    }

                                    // --------- NOSTR MESSAGES ------------
                                    Message::FetchRelayServer(url) => {
                                        to_nostr_channel(&mut client_in_sender, NostrInput::FetchRelayServer(url))
                                    }
                                    Message::FetchRelayServers => {
                                        to_nostr_channel(&mut client_in_sender, NostrInput::FetchRelayServers)
                                    }
                                    Message::CreateChannel => {
                                        to_nostr_channel(&mut client_in_sender, NostrInput::CreateChannel)
                                    }
                                    Message::ConnectToRelay(db_relay) => {
                                        match DbEvent::fetch_last(&database.pool).await {
                                            Ok(last_event) => {
                                                to_nostr_channel(
                                                    &mut client_in_sender,
                                                    NostrInput::ConnectToRelay((db_relay, last_event))
                                                )
                                            },
                                            Err(e) => Event::Error(e.to_string())
                                        }
                                    }
                                    // ----- TO BACKEND CHANNEL -----
                                    // -----------------------------------
                                    Message::PrepareClient => {
                                        match prepare_client(&database.pool).await {
                                            Ok(input) => {
                                                to_nostr_channel(
                                                    &mut client_in_sender,
                                                    input,
                                                )
                                            },
                                            Err(e) => Event::Error(e.to_string())
                                        }
                                    }
                                    // ---- NOSTR ----
                                    Message::AddRelay(db_relay) => {
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::AddRelay(db_relay)
                                        )
                                    }
                                    Message::DeleteRelay(db_relay) => {
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::DeleteRelay(db_relay)
                                        )
                                    }
                                    Message::ToggleRelayRead((db_relay, read)) => {
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::ToggleRelayRead((db_relay, read))
                                        )
                                    }
                                    Message::ToggleRelayWrite((db_relay, write)) => {
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::ToggleRelayWrite((db_relay, write))
                                        )
                                    }

                                    Message::SendContactListToRelay((db_relay, list)) => {
                                        // to_backend_channel(
                                        //     &mut backend_sender,
                                        //     send_contact_list_to(&keys, &nostr_client, &db_relay.url, &list),
                                        // ).await
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::SendContactListToRelay((db_relay, list))
                                        )
                                    }

                                    Message::SendDMTo((contact, msg)) => {
                                        // to_backend_channel(
                                        //     &mut backend_sender,
                                        //     send_dm(&nostr_client, &keys, &contact, &msg),
                                        // )
                                        // .await
                                        to_nostr_channel(
                                            &mut client_in_sender,
                                            NostrInput::SendDMTo((contact, msg))
                                        )
                                    }
                                }
                            }

                            notification = notifications_stream.select_next_some() => {
                                // Cria uma tarefa separada para processar a notificação
                                let backend_input = match notification {
                                    RelayPoolNotification::Event(relay_url, event) => {
                                        backend::BackEndInput::StoreEvent((relay_url, event))
                                    },
                                    RelayPoolNotification::Message(relay_url, msg) => {
                                        backend::BackEndInput::StoreRelayMessage((relay_url, msg))
                                    }
                                    RelayPoolNotification::Shutdown => {
                                        backend::BackEndInput::Shutdown
                                    }
                                };

                                if let Err(e) = backend_sender.try_send(backend_input) {
                                    tracing::error!("{}", e);
                                }

                                Event::None
                            },
                        };

                        (
                            event,
                            State::Connected {
                                notifications_stream,
                                in_memory,
                                database: Some(database),
                                receiver,
                                keys,
                                backend_sender,
                                backend_receiver,
                                client_in_receiver,
                                client_in_sender,
                                client_out_receiver,
                                client_out_sender,
                                client_wrapper,
                            },
                        )
                    }
                    None => (Event::Disconnected, State::Disconnected { in_memory, keys }),
                },
            }
        },
    );

    vec![database_sub]
}

// ---------------------------------

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

async fn process_async_with_event<E>(operation: impl Future<Output = Result<Event, E>>) -> Event
where
    E: std::error::Error,
{
    match operation.await {
        Ok(result) => result,
        Err(e) => Event::Error(e.to_string()),
    }
}

fn to_nostr_channel(client_sender: &mut mpsc::Sender<NostrInput>, input: NostrInput) -> Event {
    match client_sender.try_send(input) {
        Ok(_) => Event::NostrLoading,
        Err(e) => Event::Error(e.to_string()),
    }
}
fn to_backend_channel(
    backend_channel: &mut mpsc::Sender<BackEndInput>,
    input: BackEndInput,
) -> Event {
    match backend_channel.try_send(input) {
        Ok(_) => Event::None,
        Err(e) => Event::Error(e.to_string()),
    }
}

const IN_MEMORY: bool = false;
