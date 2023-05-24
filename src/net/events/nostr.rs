use sqlx::SqlitePool;

use crate::db::{DbContact, DbEvent, DbRelay, UserConfig};
use crate::error::Error;
use crate::net::operations::contact::get_contact_list_profile;
use crate::net::operations::relay::{
    add_relay, add_relays_and_connect, connect_to_relay, create_channel, delete_relay,
    fetch_relay_server, fetch_relay_servers, toggle_read_for_relay, toggle_write_for_relay,
};
use crate::net::operations::subscribe::{request_events_of, send_event_to_relays};
use async_stream::stream;
use futures::channel::mpsc;
use futures::{Future, Stream, StreamExt};
use iced::futures::stream::Fuse;
use nostr::Keys;
use nostr_sdk::{Client, Options, RelayPoolNotification};
use std::pin::Pin;

use super::{backend::BackEndInput, Event};

#[derive(Debug, Clone)]
pub enum NostrInput {
    PrepareClient {
        relays: Vec<DbRelay>,
        last_event: Option<DbEvent>,
        contact_list: Vec<DbContact>,
    },
    FetchRelayServer(nostr::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay((DbRelay, Option<DbEvent>)),
    SendEventToRelays(nostr::Event),
    GetContactListProfiles(Vec<DbContact>),
    CreateChannel,
    RequestEventsOf((DbRelay, Vec<DbContact>)),
}

pub enum NostrOutput {
    Ok(Event),
    Error(Box<dyn std::error::Error + Send>),
    ToBackend(BackEndInput),
}

impl From<Box<dyn std::error::Error + Send>> for NostrOutput {
    fn from(error: Box<dyn std::error::Error + Send>) -> Self {
        NostrOutput::Error(error)
    }
}

impl From<Event> for NostrOutput {
    fn from(event: Event) -> Self {
        NostrOutput::Ok(event)
    }
}

impl From<BackEndInput> for NostrOutput {
    fn from(input: BackEndInput) -> Self {
        NostrOutput::ToBackend(input)
    }
}

pub struct NostrSdkWrapper {
    client: Client,
}

impl NostrSdkWrapper {
    pub async fn new(
        keys: &Keys,
    ) -> (
        Self,
        Fuse<Pin<Box<dyn Stream<Item = RelayPoolNotification> + Send>>>,
    ) {
        let (client, notifications_stream) = client_with_stream(&keys).await;
        (Self { client }, notifications_stream)
    }
    pub async fn logout(self) -> Result<(), Error> {
        tracing::info!("Nostr Client Logging out");
        self.client.shutdown().await?;
        Ok(())
    }
    pub async fn process_in(
        &self,
        channel: &mut mpsc::Sender<NostrOutput>,
        keys: &Keys,
        input: NostrInput,
    ) {
        let client = self.client.clone();
        let mut channel = channel.clone();
        match input {
            NostrInput::RequestEventsOf((db_relay, contact_list)) => {
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        request_events_of(&client, &keys_1, db_relay, contact_list),
                        &mut channel,
                    )
                    .await;
                });
            }

            // NostrInput::GetContactProfile(db_contact) => {
            //     tokio::spawn(async move {
            //         run_and_send(get_contact_profile(&client, db_contact), &mut channel).await;
            //     });
            // }
            NostrInput::GetContactListProfiles(db_contacts) => {
                tokio::spawn(async move {
                    run_and_send(get_contact_list_profile(&client, db_contacts), &mut channel)
                        .await;
                });
            }

            NostrInput::SendEventToRelays(ns_event) => {
                tokio::spawn(async move {
                    match send_event_to_relays(&client, &mut channel, ns_event).await {
                        Ok(event) => {
                            if let Err(e) = channel.try_send(event.into()) {
                                tracing::error!("Error sending event to nostr output: {}", e);
                            }
                        }
                        Err(e) => {
                            if let Err(e) = channel.try_send(NostrOutput::Error(Box::new(e))) {
                                tracing::error!("Error sending event to nostr output: {}", e);
                            }
                        }
                    }
                });
            }

            NostrInput::PrepareClient {
                relays,
                last_event,
                contact_list,
            } => {
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        add_relays_and_connect(&client, &keys_1, &relays, last_event, contact_list),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::FetchRelayServer(url) => {
                tokio::spawn(async move {
                    run_and_send(fetch_relay_server(&client, &url), &mut channel).await;
                });
            }
            NostrInput::FetchRelayServers => {
                tokio::spawn(async move {
                    run_and_send(fetch_relay_servers(&client), &mut channel).await;
                });
            }
            NostrInput::AddRelay(db_relay) => {
                tokio::spawn(async move {
                    run_and_send(add_relay(&client, db_relay), &mut channel).await;
                });
            }
            NostrInput::DeleteRelay(db_relay) => {
                tokio::spawn(async move {
                    run_and_send(delete_relay(&client, db_relay), &mut channel).await;
                });
            }
            NostrInput::ToggleRelayRead((db_relay, read)) => {
                tokio::spawn(async move {
                    run_and_send(toggle_read_for_relay(&client, db_relay, read), &mut channel)
                        .await;
                });
            }
            NostrInput::ToggleRelayWrite((db_relay, write)) => {
                tokio::spawn(async move {
                    run_and_send(
                        toggle_write_for_relay(&client, db_relay, write),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::ConnectToRelay((db_relay, last_event)) => {
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        connect_to_relay(&client, &keys_1, db_relay, last_event),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::CreateChannel => {
                tokio::spawn(async move {
                    run_and_send(create_channel(&client), &mut channel).await;
                });
            }
        }
    }
}

pub async fn prepare_client(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
) -> Result<NostrInput, Error> {
    tracing::debug!("prepare_client");
    tracing::info!("Fetching relays and last event to nostr client");
    let relays = DbRelay::fetch(pool).await?;
    let last_event = DbEvent::fetch_last_kind(pool, nostr::Kind::EncryptedDirectMessage).await?;
    let contact_list = DbContact::fetch(pool, cache_pool).await?;

    UserConfig::store_first_login(pool).await?;

    Ok(NostrInput::PrepareClient {
        relays,
        last_event,
        contact_list,
    })
}

pub async fn client_with_stream(
    keys: &Keys,
) -> (
    Client,
    Fuse<Pin<Box<dyn Stream<Item = RelayPoolNotification> + Send>>>,
) {
    // Create new client
    tracing::debug!("Creating Nostr Client");
    let nostr_client = Client::with_opts(
        keys,
        Options::new()
            .wait_for_connection(false)
            .wait_for_send(false)
            .wait_for_subscription(false),
    );

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

async fn run_and_send<T, E>(
    f: impl Future<Output = Result<T, E>>,
    channel: &mut mpsc::Sender<NostrOutput>,
) where
    T: Into<NostrOutput>,
    E: std::error::Error + Send + 'static,
{
    let result = f.await;

    match result {
        Ok(t) => {
            if let Err(e) = channel.try_send(t.into()) {
                tracing::error!("{}", e);
            }
        }
        Err(e) => {
            if let Err(e) = channel.try_send(NostrOutput::Error(Box::new(e))) {
                tracing::error!("{}", e);
            }
        }
    }
}
