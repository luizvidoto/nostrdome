use async_stream::stream;
use futures::channel::mpsc;
use futures::{Future, Stream, StreamExt};
use iced::futures::stream::Fuse;
use nostr_sdk::{Client, Filter, Keys, RelayPoolNotification, Timestamp};
use std::pin::Pin;

use crate::error::Error;
use crate::net::logic::request_events;
use crate::{
    db::{DbContact, DbEvent, DbRelay},
    net::logic::{
        add_relay, add_relays_and_connect, connect_to_relay, create_channel, delete_relay,
        fetch_relay_server, fetch_relay_servers, send_contact_list_to, toggle_read_for_relay,
        toggle_write_for_relay,
    },
};

use super::{backend::BackEndInput, Event};

#[derive(Debug, Clone)]
pub enum NostrInput {
    SendDMToRelays(DbEvent),
    RequestEvents(Option<DbEvent>),
    PrepareClient {
        relays: Vec<DbRelay>,
        last_event: Option<DbEvent>,
    },
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay((DbRelay, Option<DbEvent>)),
    SendContactListToRelay((DbRelay, Vec<DbContact>)),
    CreateChannel,
}

// impl TryFrom<Message> for NostrInput {
//     type Error = ConversionError;

//     fn try_from(message: Message) -> Result<Self, Self::Error> {
//         match message {
//             Message::PrepareClient { relays, last_event } => {
//                 Ok(NostrInput::PrepareClient { relays, last_event })
//             }
//             Message::FetchRelayServer(url) => Ok(Self::FetchRelayServer(url)),
//             Message::FetchRelayServers => Ok(Self::FetchRelayServers),
//             Message::AddRelay(db_relay) => Ok(Self::AddRelay(db_relay)),
//             Message::DeleteRelay(db_relay) => Ok(Self::DeleteRelay(db_relay)),
//             Message::ToggleRelayRead((db_relay, read)) => {
//                 Ok(Self::ToggleRelayRead((db_relay, read)))
//             }
//             Message::ToggleRelayWrite((db_relay, write)) => {
//                 Ok(Self::ToggleRelayWrite((db_relay, write)))
//             }
//             Message::ConnectToRelay((db_relay, last_event)) => {
//                 Ok(Self::ConnectToRelay((db_relay, last_event)))
//             }
//             Message::SendDMTo((db_contact, content)) => Ok(Self::SendDMTo((db_contact, content))),
//             Message::SendContactListToRelay((db_relay, list)) => {
//                 Ok(Self::SendContactListToRelay((db_relay, list)))
//             }
//             Message::CreateChannel => Ok(Self::CreateChannel),
//             other => Error::UnsupportedMessage(other),
//         }
//     }
// }

pub enum NostrOutput {
    Ok(Event),
    Error(Box<dyn std::error::Error + Send>),
    ToBackend(BackEndInput),
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
    pub async fn process_in(
        &self,
        channel: &mut mpsc::Sender<NostrOutput>,
        keys: &Keys,
        input: NostrInput,
    ) {
        let client = self.client.clone();
        let mut channel = channel.clone();
        match input {
            NostrInput::SendDMToRelays(db_event) => {
                tracing::info!("SendDMToRelays");
                tokio::spawn(async move {
                    run_and_send(send_dm_to_relays(&client, db_event), &mut channel).await;
                });
            }
            NostrInput::RequestEvents(last_event) => {
                tracing::info!("RequestEvents");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        request_events(&client, &keys_1.public_key(), last_event),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::PrepareClient { relays, last_event } => {
                tracing::info!("PrepareClient");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        add_relays_and_connect(&client, &keys_1, &relays, last_event),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::FetchRelayServer(url) => {
                tracing::info!("FetchRelayServer: {}", url);
                tokio::spawn(async move {
                    run_and_send(fetch_relay_server(&client, &url), &mut channel).await;
                });
            }
            NostrInput::FetchRelayServers => {
                tracing::info!("FetchRelayServers");
                tokio::spawn(async move {
                    run_and_send(fetch_relay_servers(&client), &mut channel).await;
                });
            }
            NostrInput::AddRelay(db_relay) => {
                tracing::info!("AddRelay");
                tokio::spawn(async move {
                    run_and_send(add_relay(&client, db_relay), &mut channel).await;
                });
            }
            NostrInput::DeleteRelay(db_relay) => {
                tracing::info!("DeleteRelay");
                tokio::spawn(async move {
                    run_and_send(delete_relay(&client, db_relay), &mut channel).await;
                });
            }
            NostrInput::ToggleRelayRead((db_relay, read)) => {
                tracing::info!("ToggleRelayRead");
                tokio::spawn(async move {
                    run_and_send(toggle_read_for_relay(&client, db_relay, read), &mut channel)
                        .await;
                });
            }
            NostrInput::ToggleRelayWrite((db_relay, write)) => {
                tracing::info!("ToggleRelayWrite");
                tokio::spawn(async move {
                    run_and_send(
                        toggle_write_for_relay(&client, db_relay, write),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::ConnectToRelay((db_relay, last_event)) => {
                tracing::info!("ConnectToRelay");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        connect_to_relay(&client, &keys_1, db_relay, last_event),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::SendContactListToRelay((db_relay, list)) => {
                tracing::info!("SendContactListToRelay");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        send_contact_list_to(&keys_1, &client, &db_relay.url, &list),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::CreateChannel => {
                tracing::info!("CreateChannel");
                tokio::spawn(async move {
                    run_and_send(create_channel(&client), &mut channel).await;
                });
            }
        }
    }
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

pub async fn client_with_stream(
    keys: &Keys,
) -> (
    Client,
    Fuse<Pin<Box<dyn Stream<Item = RelayPoolNotification> + Send>>>,
) {
    // Create new client
    tracing::info!("Creating Nostr Client");
    let nostr_client = Client::new(keys);

    let sent_msgs_sub_future = Filter::new()
        .author(keys.public_key().to_string())
        .since(Timestamp::now());
    let recv_msgs_sub_future = Filter::new()
        .pubkey(keys.public_key().to_owned())
        .since(Timestamp::now());

    nostr_client
        .subscribe(vec![sent_msgs_sub_future, recv_msgs_sub_future])
        .await;

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

async fn send_dm_to_relays(client: &Client, event: DbEvent) -> Result<Event, Error> {
    let event_id = client.send_event(event.into()).await?;
    Ok(Event::SentDirectMessage(event_id))
}

// fn spawn_and_send<F>(f: F) -> impl FnOnce(Client, mpsc::Sender<NostrOutput>) + Send
// where
//     F: FnOnce(&Client) -> Pin<Box<dyn Future<Output = NostrOutput> + Send>> + Send + Sync + 'static,
// {
//     move |client: Client, mut channel: mpsc::Sender<NostrOutput>| {
//         tokio::spawn(async move {
//             let output = (f)(&client).await;
//             match channel.try_send(output) {
//                 Ok(_) => (),
//                 Err(e) => tracing::error!("{}", e),
//             };
//         });
//     }
// }
// async fn run_and_send<T, E>(
//     f: impl Future<Output = Result<T, E>>,
//     channel: &mut mpsc::Sender<NostrOutput>,
// ) where
//     T: Into<NostrOutput>,
//     E: std::error::Error + Send + 'static,
// {
//     let result = f.await;

//     match result {
//         Ok(t) => {
//             if let Err(e) = channel.try_send(t.into()) {
//                 tracing::error!("{}", e);
//             }
//         }
//         Err(e) => {
//             if let Err(e) = channel.try_send(NostrOutput::Err(Box::new(e))) {
//                 tracing::error!("{}", e);
//             }
//         }
//     }
// }
