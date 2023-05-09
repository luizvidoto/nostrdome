use std::str::FromStr;
use std::time::Duration;
use url::Url;

use async_stream::stream;
use futures::channel::mpsc;
use futures::{Future, Stream, StreamExt};
use iced::futures::stream::Fuse;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    Client, Contact, EventBuilder, Filter, Keys, Kind, Metadata, RelayPoolNotification, Timestamp,
};
use std::pin::Pin;

use crate::db::{DbContact, DbEvent, DbRelay};
use crate::error::Error;

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
    GetContactProfile(DbContact),
    SendProfile(Metadata),
    CreateChannel,
    RequestEventsOf(DbRelay),
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
        tracing::info!("Client Logging out");
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
            NostrInput::RequestEventsOf(db_relay) => {
                tracing::info!("RequestEventsOf");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(
                        request_events_of(&client, db_relay, &keys_1.public_key()),
                        &mut channel,
                    )
                    .await;
                });
            }
            NostrInput::SendProfile(metadata) => {
                tracing::info!("SendProfile");
                let keys_1 = keys.clone();
                tokio::spawn(async move {
                    run_and_send(send_profile(&client, &keys_1, metadata), &mut channel).await;
                });
            }
            NostrInput::GetContactProfile(db_contact) => {
                tracing::info!("GetContactProfile");
                tokio::spawn(async move {
                    run_and_send(get_contact_profile(&client, db_contact), &mut channel).await;
                });
            }
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

    // let sent_msgs_sub_future = Filter::new()
    //     .author(keys.public_key().to_string())
    //     .since(Timestamp::now());
    // let recv_msgs_sub_future = Filter::new()
    //     .pubkey(keys.public_key().to_owned())
    //     .since(Timestamp::now());
    // nostr_client
    //     .subscribe(vec![sent_msgs_sub_future, recv_msgs_sub_future])
    //     .await;

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

async fn get_contact_profile(client: &Client, db_contact: DbContact) -> Result<Event, Error> {
    let filter = Filter::new()
        .author(db_contact.pubkey().to_string())
        .kind(Kind::Metadata)
        .limit(1);
    let timeout = Some(Duration::from_secs(30));
    client.get_events_of(vec![filter], timeout).await?;
    Ok(Event::RequestedMetadata(db_contact))
}

async fn send_profile(client: &Client, keys: &Keys, meta: Metadata) -> Result<BackEndInput, Error> {
    let builder = EventBuilder::set_metadata(meta);
    let event = builder.to_event(keys)?;
    client.send_event(event.clone()).await?;
    Ok(BackEndInput::StorePendingEvent(event))
}

pub async fn create_channel(client: &Client) -> Result<Event, Error> {
    let metadata = Metadata::new()
        .about("Channel about cars")
        .display_name("Best Cars")
        .name("Best Cars")
        .banner(Url::from_str("https://picsum.photos/seed/picsum/800/300")?)
        .picture(Url::from_str("https://picsum.photos/seed/picsum/200/300")?)
        .website(Url::from_str("https://picsum.photos/seed/picsum/200/300")?);

    let event_id = client.new_channel(metadata).await?;

    Ok(Event::ChannelCreated(event_id))
}

pub async fn send_contact_list_to(
    // pool: &SqlitePool,
    keys: &Keys,
    client: &Client,
    url: &Url,
    list: &[DbContact],
) -> Result<BackEndInput, Error> {
    // let list = DbContact::fetch(pool).await?;
    let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();

    let builder = EventBuilder::set_contact_list(c_list);
    let event = builder.to_event(keys)?;

    let _event_id = client.send_event_to(url.to_owned(), event.clone()).await?;

    Ok(BackEndInput::StorePendingEvent(event))
}

pub fn build_dm(keys: &Keys, db_contact: &DbContact, content: &str) -> Result<BackEndInput, Error> {
    let builder =
        EventBuilder::new_encrypted_direct_msg(&keys, db_contact.pubkey().to_owned(), content)?;
    let event = builder.to_event(keys)?;
    Ok(BackEndInput::StorePendingEvent(event))
}

pub async fn add_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::info!("Adding relay to client: {}", db_relay.url);
    client.add_relay(db_relay.url.as_str(), None).await?;
    Ok(BackEndInput::AddRelayToDb(db_relay))
}

pub async fn fetch_relay_server(client: &Client, url: &Url) -> Result<Event, Error> {
    let relay = client.relays().await.get(url).cloned();
    Ok(Event::GotRelayServer(relay))
}

pub async fn fetch_relay_servers(client: &Client) -> Result<Event, Error> {
    let relays: Vec<nostr_sdk::Relay> = client.relays().await.values().cloned().collect();
    Ok(Event::GotRelayServers(relays))
}

pub async fn add_relays_and_connect(
    client: &Client,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    // Received from BackEndEvent::PrepareClient
    tracing::info!("Adding relays to client");
    for r in relays {
        match client.add_relay(&r.url.to_string(), None).await {
            Ok(_) => tracing::info!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::info!("Connecting to relays");
    let client_c = client.clone();
    tokio::spawn(async move { client_c.connect().await });

    request_events(&client, &keys.public_key(), last_event).await?;

    Ok(Event::FinishedPreparing)
}

pub async fn connect_to_relay(
    client: &Client,
    _keys: &Keys,
    db_relay: DbRelay,
    _last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    let event = if let Some(relay) = client
        .relays()
        .await
        .values()
        .find(|r| &r.url() == &db_relay.url)
    {
        if nostr_sdk::RelayStatus::Connected != relay.status().await {
            tracing::info!("Trying to connect to relay: {}", db_relay.url);
            let client = client.clone();
            let db_relay_clone = db_relay.clone();
            tokio::spawn(async move {
                if let Err(e) = client.connect_relay(db_relay_clone.url.as_str()).await {
                    tracing::error!("[client.connect_relay] ERROR: {}", e);
                }
            });
            Event::RelayConnected(db_relay)
        } else {
            Event::None
        }
    } else {
        tracing::warn!("Relay not found on client: {}", db_relay.url);
        // add_relay(client, db_relay).await?
        Event::None
    };

    Ok(event)
}

async fn request_events_of(
    client: &Client,
    db_relay: DbRelay,
    own_public_key: &XOnlyPublicKey,
) -> Result<Event, Error> {
    if let Some(relay) = client.relays().await.get(&db_relay.url) {
        let from_me = Filter::new()
            .kinds(nostr_kinds())
            .author(own_public_key.to_string())
            .until(Timestamp::now());
        let to_me = Filter::new()
            .kinds(nostr_kinds())
            .pubkey(own_public_key.to_owned())
            .until(Timestamp::now());
        let timeout = Some(Duration::from_secs(10));
        relay.req_events_of(vec![from_me, to_me], timeout);
        Ok(Event::RequestedEventsOf(db_relay))
    } else {
        Ok(Event::None)
    }
}

pub async fn request_events(
    client: &Client,
    public_key: &XOnlyPublicKey,
    last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    tracing::info!("Requesting events from relays");
    let last_timestamp_secs: u64 = last_event
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.created_at - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0);
    tracing::info!("last timestamp: {}", last_timestamp_secs);

    let client = client.clone();
    let pk = public_key.clone();
    tokio::spawn(async move {
        // if has last_event, request events since last_event.timestamp
        // else request events since 0
        let sent_msgs_sub_past = Filter::new()
            .kinds(nostr_kinds())
            .author(pk.to_string())
            .since(Timestamp::from(last_timestamp_secs))
            .until(Timestamp::now());
        let recv_msgs_sub_past = Filter::new()
            .kinds(nostr_kinds())
            .pubkey(pk.to_owned())
            .since(Timestamp::from(last_timestamp_secs))
            .until(Timestamp::now());
        let timeout = Duration::from_secs(10);
        client
            .req_events_of(vec![sent_msgs_sub_past, recv_msgs_sub_past], Some(timeout))
            .await;
    });

    Ok(Event::RequestedEvents)
}

pub async fn toggle_read_for_relay(
    client: &Client,
    db_relay: DbRelay,
    read: bool,
) -> Result<BackEndInput, Error> {
    let mut relays = client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_read(read)
    }
    Ok(BackEndInput::ToggleRelayRead((db_relay, read)))
}
pub async fn toggle_write_for_relay(
    client: &Client,
    db_relay: DbRelay,
    write: bool,
) -> Result<BackEndInput, Error> {
    let mut relays = client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_write(write)
    }
    Ok(BackEndInput::ToggleRelayWrite((db_relay, write)))
}

pub async fn delete_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    client.remove_relay(db_relay.url.as_str()).await?;
    Ok(BackEndInput::DeleteRelayFromDb(db_relay))
}

pub fn nostr_kinds() -> Vec<Kind> {
    [
        Kind::Metadata,
        Kind::EncryptedDirectMessage,
        Kind::RecommendRelay,
        Kind::ContactList,
    ]
    .to_vec()
}
