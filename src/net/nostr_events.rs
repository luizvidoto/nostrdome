use std::str::FromStr;
use std::time::Duration;

use nostr::secp256k1::XOnlyPublicKey;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use url::Url;

use crate::db::{DbContact, DbEvent, DbRelay, UserConfig};
use crate::error::Error;
use crate::net::operations::contact::get_contact_list_profile;
use crate::net::{BackEndInput, BackendEvent};
use nostr::{ClientMessage, Filter, Keys, Kind, Metadata, Timestamp};
use nostr_sdk::{Client, Options, Relay, RelayOptions, RelayPoolNotification, RelayStatus};

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
    ToFrontEnd(BackendEvent),
    ToBackend(BackEndInput),
}

impl From<BackendEvent> for NostrOutput {
    fn from(event: BackendEvent) -> Self {
        NostrOutput::ToFrontEnd(event)
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
    pub async fn new(keys: &Keys) -> Self {
        let client = client(&keys).await;
        Self { client }
    }
    pub fn notifications(&self) -> broadcast::Receiver<RelayPoolNotification> {
        self.client.notifications()
    }
    pub async fn logout(self) -> Result<(), Error> {
        tracing::info!("Nostr Client Logging out");
        self.client.shutdown().await?;
        Ok(())
    }
    pub async fn process_in(
        &self,
        keys: &Keys,
        input: NostrInput,
    ) -> Result<Option<NostrOutput>, Error> {
        let event_opt = match input {
            NostrInput::RequestEventsOf((db_relay, contact_list)) => Some(
                request_events_of(&self.client, &keys, db_relay, contact_list)
                    .await?
                    .into(),
            ),
            NostrInput::GetContactListProfiles(db_contacts) => Some(
                get_contact_list_profile(&self.client, db_contacts)
                    .await?
                    .into(),
            ),

            NostrInput::SendEventToRelays(ns_event) => {
                Some(send_event_to_relays(&self.client, ns_event).await.into())
            }

            NostrInput::PrepareClient {
                relays,
                last_event,
                contact_list,
            } => Some(
                add_relays_and_connect(&self.client, &keys, &relays, last_event, contact_list)
                    .await
                    .into(),
            ),
            NostrInput::FetchRelayServer(url) => {
                Some(fetch_relay_server(&self.client, &url).await.into())
            }
            NostrInput::FetchRelayServers => Some(fetch_relay_servers(&self.client).await.into()),
            NostrInput::AddRelay(db_relay) => Some(add_relay(&self.client, db_relay).await?.into()),
            NostrInput::DeleteRelay(db_relay) => {
                Some(delete_relay(&self.client, db_relay).await?.into())
            }
            NostrInput::ToggleRelayRead((db_relay, read)) => Some(
                toggle_read_for_relay(&self.client, db_relay, read)
                    .await
                    .into(),
            ),
            NostrInput::ToggleRelayWrite((db_relay, write)) => Some(
                toggle_write_for_relay(&self.client, db_relay, write)
                    .await
                    .into(),
            ),
            NostrInput::ConnectToRelay((db_relay, last_event)) => {
                connect_to_relay(&self.client, &keys, db_relay, last_event)
                    .await?
                    .map(|event| event.into())
            }
            NostrInput::CreateChannel => Some(create_channel(&self.client).await?.into()),
        };

        Ok(event_opt)
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

pub async fn client(keys: &Keys) -> Client {
    // Create new client
    tracing::debug!("Creating Nostr Client");
    let nostr_client = Client::with_opts(
        keys,
        Options::new()
            .wait_for_connection(false)
            .wait_for_send(false)
            .wait_for_subscription(false),
    );
    nostr_client
}

pub async fn create_channel(client: &Client) -> Result<BackendEvent, Error> {
    tracing::debug!("create_channel");
    let metadata = Metadata::new()
        .about("Channel about cars")
        .display_name("Best Cars")
        .name("Best Cars")
        .banner(Url::from_str("https://picsum.photos/seed/picsum/800/300")?)
        .picture(Url::from_str("https://picsum.photos/seed/picsum/200/300")?)
        .website(Url::from_str("https://picsum.photos/seed/picsum/200/300")?);

    let event_id = client.new_channel(metadata).await?;

    Ok(BackendEvent::ChannelCreated(event_id))
}

pub async fn add_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("Adding relay to client: {}", db_relay.url);
    let opts = RelayOptions::new(db_relay.read, db_relay.write);
    client
        .add_relay_with_opts(db_relay.url.as_str(), None, opts)
        .await?;
    Ok(BackEndInput::AddRelayToDb(db_relay))
}

pub async fn fetch_relay_server(client: &Client, url: &nostr::Url) -> BackendEvent {
    tracing::trace!("fetch_relay_server");
    let relay = client.relays().await.get(url).cloned();
    BackendEvent::GotRelayServer(relay)
}

pub async fn fetch_relay_servers(client: &Client) -> BackendEvent {
    tracing::trace!("fetch_relay_servers");
    let relays: Vec<Relay> = client.relays().await.values().cloned().collect();
    BackendEvent::GotRelayServers(relays)
}

pub async fn add_relays_and_connect(
    client: &Client,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
    contact_list: Vec<DbContact>,
) -> BackEndInput {
    tracing::debug!("add_relays_and_connect");
    // Received from BackEndEvent::PrepareClient
    tracing::debug!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        let opts = RelayOptions::new(r.read, r.write);
        match client
            .add_relay_with_opts(&r.url.to_string(), None, opts)
            .await
        {
            Ok(_) => tracing::debug!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::debug!("Connecting to relays");
    client.connect().await;

    subscribe_to_events(client, keys, last_event, contact_list).await;

    BackEndInput::FinishedPreparingNostr
}

pub async fn connect_to_relay(
    client: &Client,
    _keys: &Keys,
    db_relay: DbRelay,
    _last_event: Option<DbEvent>,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("connect_to_relay");
    let event = if let Some(relay) = client
        .relays()
        .await
        .values()
        .find(|r| &r.url() == &db_relay.url)
    {
        if RelayStatus::Connected != relay.status().await {
            tracing::info!("Trying to connect to relay: {}", db_relay.url);
            let client = client.clone();
            let db_relay_clone = db_relay.clone();
            client.connect_relay(db_relay_clone.url.as_str()).await?;
            Some(BackendEvent::RelayConnected(db_relay))
        } else {
            None
        }
    } else {
        tracing::warn!("Relay not found on client: {}", db_relay.url);
        // add_relay(client, db_relay).await?
        return Err(Error::RelayNotFound(db_relay.url.to_string()));
    };

    Ok(event)
}

pub async fn toggle_read_for_relay(client: &Client, db_relay: DbRelay, read: bool) -> BackEndInput {
    tracing::debug!("toggle_read_for_relay");
    let mut relays = client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_read(read)
    }
    BackEndInput::ToggleRelayRead((db_relay, read))
}
pub async fn toggle_write_for_relay(
    client: &Client,
    db_relay: DbRelay,
    write: bool,
) -> BackEndInput {
    tracing::debug!("toggle_write_for_relay");
    let mut relays = client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_write(write)
    }
    BackEndInput::ToggleRelayWrite((db_relay, write))
}

pub async fn delete_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("delete_relay");
    client.remove_relay(db_relay.url.as_str()).await?;
    Ok(BackEndInput::DeleteRelayFromDb(db_relay))
}

pub async fn request_events_of(
    client: &Client,
    keys: &Keys,
    db_relay: DbRelay,
    contact_list: Vec<DbContact>,
) -> Result<BackendEvent, Error> {
    tracing::debug!("Requesting events of {}", db_relay.url);
    tracing::debug!("contact_list: {}", contact_list.len());
    if let Some(relay) = client.relays().await.get(&db_relay.url) {
        let filters = make_nostr_filters(keys.public_key().clone(), None, &contact_list);
        let timeout = Some(Duration::from_secs(10));
        relay.req_events_of(filters, timeout);
    }
    Ok(BackendEvent::RequestedEventsOf(db_relay))
}

pub async fn subscribe_to_events(
    client: &Client,
    keys: &Keys,
    last_event: Option<DbEvent>,
    contact_list: Vec<DbContact>,
) -> BackendEvent {
    tracing::info!("Subscribing to events");
    let filters = make_nostr_filters(keys.public_key(), last_event, &contact_list);
    client.subscribe(filters).await;
    BackendEvent::SubscribedToEvents
}

fn make_nostr_filters(
    public_key: XOnlyPublicKey,
    last_event: Option<DbEvent>,
    contact_list: &[DbContact],
) -> Vec<Filter> {
    tracing::debug!("make_nostr_filters");
    let last_timestamp_secs: u64 = last_event
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.local_creation - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0);
    tracing::info!("last event timestamp: {}", last_timestamp_secs);

    let all_pubkeys = contact_list
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();
    let metadata_filter = Filter::new()
        .authors(all_pubkeys)
        .kind(Kind::Metadata)
        .since(Timestamp::from(last_timestamp_secs));
    let sent_msgs_sub_past = Filter::new()
        .kinds(nostr_kinds())
        .author(public_key.to_string())
        .since(Timestamp::from(last_timestamp_secs));
    let recv_msgs_sub_past = Filter::new()
        .kinds(nostr_kinds())
        .pubkey(public_key)
        .since(Timestamp::from(last_timestamp_secs));

    vec![sent_msgs_sub_past, recv_msgs_sub_past, metadata_filter]
}

pub async fn send_event_to_relays(client: &Client, ns_event: nostr::Event) -> BackendEvent {
    tracing::debug!("send_event_to_relays ID: {}", ns_event.id);
    tracing::debug!("{:?}", ns_event);
    for (url, relay) in client.relays().await {
        let status = relay.status().await;
        if let RelayStatus::Connected = status {
            let msg = ClientMessage::Event(Box::new(ns_event.clone()));
            tracing::debug!("To relay: {}", &url);
            if let Err(e) = relay.send_msg(msg, false).await {
                tracing::error!("Error sending event to relay: {}", e);
            }
        }
    }
    BackendEvent::SentEventToRelays(ns_event.id)
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
