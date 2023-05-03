use async_stream::stream;
use futures::channel::mpsc;
use futures::{Future, Stream, StreamExt};
use iced::futures::stream::Fuse;
use nostr_sdk::{Client, Filter, Keys, RelayPoolNotification, Timestamp};
use std::pin::Pin;

use crate::{
    db::{DbContact, DbEvent, DbRelay},
    net::logic::{
        add_relay, add_relays_and_connect, connect_to_relay, create_channel, delete_relay,
        fetch_relay_server, fetch_relay_servers, send_contact_list_to, send_dm,
        toggle_read_for_relay, toggle_write_for_relay,
    },
};

use super::{backend::BackEndInput, Event};

#[derive(Debug, Clone)]
pub enum NostrInput {
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
    SendDMTo((DbContact, String)),
    SendContactListToRelay((DbRelay, Vec<DbContact>)),
    CreateChannel,
}

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
            NostrInput::SendDMTo((db_contact, content)) => {
                tracing::info!("SendDMTo");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        send_dm(&client, &keys_1, &db_contact, &content),
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
