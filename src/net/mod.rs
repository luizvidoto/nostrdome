use crate::db::{Database, DbEvent, DbRelay};
use crate::net::events::backend::BackEndEvent;
use crate::net::logic::{delete_contact, import_contacts, insert_contact, update_contact};
use async_stream::stream;
use futures::channel::mpsc;
use futures::{Future, Stream, StreamExt};
use iced::futures::stream::Fuse;
use iced::Subscription;
use iced_native::subscription;
use logic::*;
use nostr_sdk::RelayPoolNotification;
use nostr_sdk::{Client, Keys};
use std::pin::Pin;
use std::time::Duration;

mod back_channel;
pub(crate) mod events;
mod logic;
mod messages;

pub(crate) use self::back_channel::{BackEndConnection, Connection};
use self::events::Event;
pub(crate) use messages::Message;

pub enum State {
    Disconnected {
        in_memory: bool,
        keys: Keys,
        conn: BackEndConnection<Message>,
    },
    Processing {
        in_memory: bool,
        keys: Keys,
        conn: BackEndConnection<Message>,
        database: Option<Database>,
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Option<Client>,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
    },
    Connected {
        in_memory: bool,
        keys: Keys,
        conn: BackEndConnection<Message>,
        database: Option<Database>,
        receiver: mpsc::UnboundedReceiver<Message>,
        nostr_client: Option<Client>,
        notifications_stream:
            Fuse<Pin<Box<dyn futures::Stream<Item = RelayPoolNotification> + Send>>>,
    },
}

pub fn backend_connect(keys: &Keys, conn: &BackEndConnection<Message>) -> Vec<Subscription<Event>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub = subscription::unfold(
        id,
        State::Disconnected {
            keys: keys.clone(),
            in_memory: IN_MEMORY,
            conn: conn.clone(),
        },
        |state| async move {
            match state {
                State::Disconnected {
                    keys,
                    in_memory,
                    mut conn,
                } => {
                    let (sender, receiver) = mpsc::unbounded();

                    conn.with_channel(sender);

                    let (nostr_client, notifications_stream) = client_with_stream(&keys);

                    let database =
                        match Database::new(in_memory, &keys.public_key().to_string()).await {
                            Ok(database) => database,
                            Err(e) => {
                                tracing::error!("Failed to init database");
                                tracing::error!("{}", e);
                                tracing::warn!("Trying again in 2 secs");
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                return (
                                    Event::Disconnected,
                                    State::Disconnected {
                                        keys,
                                        in_memory,
                                        conn,
                                    },
                                );
                            }
                        };

                    (
                        Event::Connected,
                        State::Processing {
                            in_memory,
                            database: Some(database),
                            nostr_client: Some(nostr_client),
                            notifications_stream,
                            receiver,
                            keys,
                            conn,
                        },
                    )
                }
                State::Processing {
                    nostr_client,
                    notifications_stream,
                    in_memory,
                    database,
                    receiver,
                    keys,
                    conn,
                } => {
                    conn.process_message_queue();
                    // Aguarde o intervalo
                    tokio::time::sleep(Duration::from_millis(APP_TICK_INTERVAL_MILLIS)).await;

                    (
                        Event::FinishedProcessing,
                        State::Connected {
                            database,
                            receiver,
                            keys,
                            conn,
                            in_memory,
                            nostr_client,
                            notifications_stream,
                        },
                    )
                }
                State::Connected {
                    nostr_client,
                    mut notifications_stream,
                    in_memory,
                    database,
                    mut receiver,
                    keys,
                    conn,
                } => match (database, nostr_client) {
                    (Some(database), Some(nostr_client)) => {
                        let event = futures::select! {
                            message = receiver.select_next_some() => {
                                let event = match message {
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
                                    Message::AddRelay(db_relay) => {
                                        process_async_with_event(
                                            add_relay(&nostr_client, db_relay),
                                        )
                                        .await
                                    }
                                    Message::DeleteRelay(db_relay) => {
                                        process_async_with_event(
                                            delete_relay(&nostr_client, db_relay),
                                        )
                                        .await
                                    }
                                    Message::FetchRelays => {
                                        process_async_with_event(
                                            fetch_relays(&database.pool)
                                        )
                                        .await
                                    }
                                    Message::ToggleRelayRead((db_relay, read)) => {
                                        process_async_with_event(
                                            toggle_read_for_relay(&nostr_client, db_relay, read),
                                        ).await
                                    }
                                    Message::ToggleRelayWrite((db_relay, write)) => {
                                        process_async_with_event(
                                            toggle_write_for_relay(&nostr_client, db_relay, write),
                                        ).await
                                    }

                                    Message::FetchMessages (contact) => {
                                        process_async_with_event(
                                            fetch_and_decrypt_chat(&keys, &database.pool, contact),
                                        )
                                        .await
                                    }
                                    // --------- NOSTR MESSAGES ------------
                                    Message::FetchRelayServer(url) => {
                                        process_async_with_event(
                                            fetch_relay_server(&nostr_client, &url),
                                        )
                                        .await
                                    }
                                    Message::FetchRelayServers => {
                                        process_async_with_event(
                                            fetch_relay_servers(&nostr_client),
                                        )
                                        .await
                                    }
                                    Message::SendContactListToRelay((db_relay, list)) => {
                                        process_async_with_event(
                                            send_contact_list_to(&keys, &nostr_client, &db_relay.url, &list),
                                        ).await
                                    }
                                    Message::CreateChannel => {
                                        process_async_with_event(
                                            create_channel(&nostr_client),
                                        ).await
                                    }
                                    Message::SendDMTo((contact, msg)) => {
                                        process_async_with_event(
                                            send_dm(&nostr_client, &keys, &contact, &msg),
                                        )
                                        .await
                                    }
                                    Message::ShowPublicKey => {
                                        let pb_key = keys.public_key();
                                        Event::GotPublicKey(pb_key)
                                    }
                                    Message::ConnectToRelay(db_relay) => {
                                        match DbEvent::fetch_last(&database.pool).await {
                                            Ok(last_event) => {
                                                process_async_with_event(
                                                    connect_relay(&nostr_client, &keys, db_relay, last_event)
                                                ).await
                                            },
                                            Err(e) => Event::Error(e.to_string())
                                        }
                                    }
                                    // -----------------------------------
                                    Message::PrepareClient => {
                                        process_async_with_event(
                                            prepare_client(&database.pool),
                                        ).await
                                    }
                                    Message::ProcessMessages => {
                                        return (Event::IsProcessing, State::Processing {
                                            nostr_client:Some(nostr_client),
                                            notifications_stream,
                                            in_memory,
                                            database: Some(database),
                                            receiver,
                                            keys,
                                            conn,
                                        });
                                    }
                                };

                                event
                            }
                            notification = notifications_stream.select_next_some() => {
                                let event = match notification {
                                    RelayPoolNotification::Event(relay_url, event) => {
                                        Event::BackEndEvent(BackEndEvent::ReceivedEvent((relay_url, event)))
                                    },
                                    RelayPoolNotification::Message(relay_url, msg) => {
                                        Event::BackEndEvent(BackEndEvent::ReceivedRelayMessage((relay_url, msg)))
                                    }
                                    RelayPoolNotification::Shutdown => {
                                        Event::Shutdown
                                    }
                                };

                                event
                            },
                        };
                        let event = if let Event::BackEndEvent(back_ev) = event {
                            match back_ev {
                                BackEndEvent::DeleteRelayFromDb(db_relay) => {
                                    process_async_with_event(db_delete_relay(
                                        &database.pool,
                                        db_relay,
                                    ))
                                    .await
                                }
                                BackEndEvent::ToggleRelayRead((mut db_relay, read)) => {
                                    db_relay.read = read;
                                    match DbRelay::update(&database.pool, &db_relay).await {
                                        Ok(_) => Event::RelayUpdated(db_relay.clone()),
                                        Err(e) => Event::Error(e.to_string()),
                                    }
                                }
                                BackEndEvent::ToggleRelayWrite((mut db_relay, write)) => {
                                    db_relay.write = write;
                                    match DbRelay::update(&database.pool, &db_relay).await {
                                        Ok(_) => Event::RelayUpdated(db_relay.clone()),
                                        Err(e) => Event::Error(e.to_string()),
                                    }
                                }
                                BackEndEvent::AddRelayToDb(db_relay) => {
                                    process_async_with_event(db_add_relay(&database.pool, db_relay))
                                        .await
                                }
                                BackEndEvent::PrepareClient { relays, last_event } => {
                                    process_async_with_event(add_relays_and_connect(
                                        &nostr_client,
                                        &keys,
                                        &relays,
                                        last_event,
                                    ))
                                    .await
                                }
                                BackEndEvent::InsertPendingEvent(nostr_event) => {
                                    process_async_fn(
                                        insert_pending_event(&database.pool, &keys, nostr_event),
                                        |event| event,
                                    )
                                    .await
                                }
                                BackEndEvent::ReceivedEvent((relay_url, nostr_event)) => {
                                    process_async_fn(
                                        received_event(
                                            &database.pool,
                                            &keys,
                                            nostr_event,
                                            &relay_url,
                                        ),
                                        |event| event,
                                    )
                                    .await
                                }
                                BackEndEvent::ReceivedRelayMessage((relay_url, relay_message)) => {
                                    process_async_fn(
                                        on_relay_message(
                                            &database.pool,
                                            &relay_url,
                                            &relay_message,
                                        ),
                                        |event| event,
                                    )
                                    .await
                                }
                            }
                        } else {
                            event
                        };
                        (
                            event,
                            State::Connected {
                                nostr_client: Some(nostr_client),
                                notifications_stream,
                                in_memory,
                                database: Some(database),
                                receiver,
                                keys,
                                conn,
                            },
                        )
                    }
                    (None, Some(nostr_client)) => {
                        match Database::new(in_memory, &keys.public_key().to_string()).await {
                            Ok(database) => (
                                Event::DbConnected,
                                State::Connected {
                                    nostr_client: Some(nostr_client),
                                    notifications_stream,
                                    in_memory,
                                    keys,
                                    conn,
                                    database: Some(database),
                                    receiver,
                                },
                            ),
                            Err(e) => {
                                tracing::error!("Failed to connect to database");
                                tracing::error!("{}", e);
                                tracing::warn!("Trying again in 2 secs");
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                (
                                    Event::DbDisconnected,
                                    State::Connected {
                                        in_memory,
                                        nostr_client: Some(nostr_client),
                                        notifications_stream,
                                        keys,
                                        conn,
                                        database: None,
                                        receiver,
                                    },
                                )
                            }
                        }
                    }
                    (Some(database), None) => {
                        let (nostr_client, notifications_stream) = client_with_stream(&keys);
                        (
                            Event::NostrConnected,
                            State::Connected {
                                nostr_client: Some(nostr_client),
                                notifications_stream,
                                in_memory,
                                keys,
                                conn,
                                database: Some(database),
                                receiver,
                            },
                        )
                    }
                    (None, None) => (
                        Event::Disconnected,
                        State::Disconnected {
                            in_memory,
                            keys,
                            conn,
                        },
                    ),
                },
            }
        },
    );

    vec![database_sub]
}

// ---------------------------------

fn client_with_stream(
    keys: &Keys,
) -> (
    Client,
    Fuse<Pin<Box<dyn Stream<Item = RelayPoolNotification> + Send>>>,
) {
    // Create new client
    tracing::info!("Creating Nostr Client");
    let nostr_client = Client::new(keys);

    let mut notifications = nostr_client.notifications();
    let notifications_stream = stream! {
        while let Ok(notification) = notifications.recv().await {
            yield notification;
        }
    }
    .boxed()
    .fuse();
    (nostr_client, notifications_stream)
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

async fn process_async_with_event<E>(operation: impl Future<Output = Result<Event, E>>) -> Event
where
    E: std::error::Error,
{
    match operation.await {
        Ok(result) => result,
        Err(e) => Event::Error(e.to_string()),
    }
}

const APP_TICK_INTERVAL_MILLIS: u64 = 50;
const IN_MEMORY: bool = false;
