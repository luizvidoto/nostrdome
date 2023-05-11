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
    PrepareClient {
        relays: Vec<DbRelay>,
        last_event: Option<DbEvent>,
        contact_list: Vec<DbContact>,
    },
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay((DbRelay, Option<DbEvent>)),
    SendEventToRelays(DbEvent),
    GetContactProfile(DbContact),
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

            NostrInput::GetContactProfile(db_contact) => {
                tokio::spawn(async move {
                    run_and_send(get_contact_profile(&client, db_contact), &mut channel).await;
                });
            }
            NostrInput::GetContactListProfiles(db_contacts) => {
                tokio::spawn(async move {
                    run_and_send(get_contact_list_profile(&client, db_contacts), &mut channel)
                        .await;
                });
            }

            NostrInput::SendEventToRelays(db_event) => {
                tokio::spawn(async move {
                    run_and_send(send_event_to_relays(&client, db_event), &mut channel).await;
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

pub async fn request_events_of(
    client: &Client,
    keys: &Keys,
    db_relay: DbRelay,
    contact_list: Vec<DbContact>,
) -> Result<Event, Error> {
    tracing::info!("Requesting events of {}", db_relay.url);
    tracing::info!("contact_list: {}", contact_list.len());
    if let Some(relay) = client.relays().await.get(&db_relay.url) {
        let filters = make_nostr_filters(keys.public_key().clone(), None, &contact_list);
        let timeout = Some(Duration::from_secs(10));
        relay.req_events_of(filters, timeout);
    }
    Ok(Event::RequestedEventsOf(db_relay))
}

pub async fn subscribe_to_events(
    client: &Client,
    keys: &Keys,
    last_event: Option<DbEvent>,
    contact_list: Vec<DbContact>,
) -> Result<Event, Error> {
    tracing::info!("Subscribing to events");

    let client = client.clone();
    let public_key = keys.public_key().clone();
    tokio::spawn(async move {
        // if has last_event, request events since last_event.timestamp
        // else request events since 0
        let filters = make_nostr_filters(public_key, last_event, &contact_list);
        client.subscribe(filters).await;
    });
    Ok(Event::SubscribedToEvents)
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
    let contact_list_filter = Filter::new()
        .authors(all_pubkeys)
        .since(Timestamp::from(last_timestamp_secs))
        .kind(Kind::Metadata);

    let sent_msgs_sub_past = Filter::new()
        .kinds(nostr_kinds())
        .author(public_key.to_string())
        .since(Timestamp::from(last_timestamp_secs));
    let recv_msgs_sub_past = Filter::new()
        .kinds(nostr_kinds())
        .pubkey(public_key)
        .since(Timestamp::from(last_timestamp_secs));

    vec![sent_msgs_sub_past, recv_msgs_sub_past, contact_list_filter]
}

pub async fn client_with_stream(
    keys: &Keys,
) -> (
    Client,
    Fuse<Pin<Box<dyn Stream<Item = RelayPoolNotification> + Send>>>,
) {
    // Create new client
    tracing::debug!("Creating Nostr Client");
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

async fn send_event_to_relays(client: &Client, db_event: DbEvent) -> Result<Event, Error> {
    tracing::debug!("send_event_to_relays: {}", db_event.event_hash);
    let event_hash = client.send_event(db_event.nostr_event()).await?;
    Ok(Event::SentEventToRelays(event_hash))
}

async fn get_contact_profile(client: &Client, db_contact: DbContact) -> Result<Event, Error> {
    tracing::debug!("get_contact_profile");
    let filter = Filter::new()
        .author(db_contact.pubkey().to_string())
        .kind(Kind::Metadata);
    let timeout = Some(Duration::from_secs(10));
    client.req_events_of(vec![filter], timeout).await;
    Ok(Event::RequestedContactProfile(db_contact))
}

async fn get_contact_list_profile(
    client: &Client,
    db_contacts: Vec<DbContact>,
) -> Result<Event, Error> {
    tracing::debug!("get_contact_list_profile: {}", db_contacts.len());
    let all_pubkeys = db_contacts
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();
    let filter = Filter::new().authors(all_pubkeys).kind(Kind::Metadata);
    let timeout = Some(Duration::from_secs(10));
    client.req_events_of(vec![filter], timeout).await;
    Ok(Event::RequestedContactListProfiles)
}

pub async fn create_channel(client: &Client) -> Result<Event, Error> {
    tracing::debug!("create_channel");
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

pub async fn add_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("add_relay");
    tracing::info!("Adding relay to client: {}", db_relay.url);
    client.add_relay(db_relay.url.as_str(), None).await?;
    Ok(BackEndInput::AddRelayToDb(db_relay))
}

pub async fn fetch_relay_server(client: &Client, url: &Url) -> Result<Event, Error> {
    tracing::debug!("fetch_relay_server");
    let relay = client.relays().await.get(url).cloned();
    Ok(Event::GotRelayServer(relay))
}

pub async fn fetch_relay_servers(client: &Client) -> Result<Event, Error> {
    tracing::debug!("fetch_relay_servers");
    let relays: Vec<nostr_sdk::Relay> = client.relays().await.values().cloned().collect();
    Ok(Event::GotRelayServers(relays))
}

pub async fn add_relays_and_connect(
    client: &Client,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
    contact_list: Vec<DbContact>,
) -> Result<BackEndInput, Error> {
    tracing::debug!("add_relays_and_connect");
    // Received from BackEndEvent::PrepareClient
    tracing::info!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        match client.add_relay(&r.url.to_string(), None).await {
            Ok(_) => tracing::info!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::info!("Connecting to relays");
    let client_c = client.clone();
    tokio::spawn(async move { client_c.connect().await });

    subscribe_to_events(client, keys, last_event, contact_list).await?;

    Ok(BackEndInput::FinishedPreparingNostr)
}

pub async fn connect_to_relay(
    client: &Client,
    _keys: &Keys,
    db_relay: DbRelay,
    _last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    tracing::debug!("connect_to_relay");
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

pub async fn toggle_read_for_relay(
    client: &Client,
    db_relay: DbRelay,
    read: bool,
) -> Result<BackEndInput, Error> {
    tracing::debug!("toggle_read_for_relay");
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
    tracing::debug!("toggle_write_for_relay");
    let mut relays = client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_write(write)
    }
    Ok(BackEndInput::ToggleRelayWrite((db_relay, write)))
}

pub async fn delete_relay(client: &Client, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("delete_relay");
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

// ------- EVENT BUILDERS ---------
//
//

pub async fn build_profile_event(keys: &Keys, metadata: Metadata) -> Result<BackEndInput, Error> {
    tracing::debug!("send_profile");
    let builder = EventBuilder::set_metadata(metadata.clone());
    let ns_event = builder.to_event(keys)?;
    Ok(BackEndInput::StorePendingMetadata((ns_event, metadata)))
}

pub async fn build_contact_list_event(
    keys: &Keys,
    list: &[DbContact],
) -> Result<BackEndInput, Error> {
    tracing::debug!("build_contact_list_event");
    let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();
    let builder = EventBuilder::set_contact_list(c_list);
    let ns_event = builder.to_event(keys)?;
    // client
    //     .send_event_to(url.to_owned(), ns_event.clone())
    //     .await?;
    Ok(BackEndInput::StorePendingContactList((
        ns_event,
        list.to_owned(),
    )))
}

pub fn build_dm(
    keys: &Keys,
    db_contact: DbContact,
    content: String,
) -> Result<BackEndInput, Error> {
    tracing::debug!("build_dm");
    let builder =
        EventBuilder::new_encrypted_direct_msg(&keys, db_contact.pubkey().to_owned(), &content)?;
    let ns_event = builder.to_event(keys)?;
    Ok(BackEndInput::StorePendingMessage {
        ns_event,
        content,
        db_contact,
    })
}

// -------  ---------
//
//
