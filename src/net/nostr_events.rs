use nostr::secp256k1::XOnlyPublicKey;
use ns_client::{NotificationEvent, RelayPool};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::db::{DbContact, DbEvent, DbRelay, UserConfig};
use crate::error::Error;
use crate::net::{BackEndInput, BackendEvent};
use nostr::{Filter, Keys, Kind, Timestamp};

#[derive(Debug, Clone)]
pub enum NostrInput {
    PrepareClient {
        relays: Vec<DbRelay>,
        last_event: Option<DbEvent>,
        contact_list: Vec<DbContact>,
    },
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    SendEventToRelays(nostr::Event),
    GetContactListProfiles(Vec<DbContact>),
    CreateChannel,
    RequestEventsOf((DbRelay, Vec<DbContact>)),
    GetRelayStatusList,
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
    client: RelayPool,
}

impl NostrSdkWrapper {
    pub async fn new(_keys: &Keys) -> Self {
        let client = RelayPool::new();
        Self { client }
    }
    pub fn notifications(&self) -> broadcast::Receiver<NotificationEvent> {
        self.client.notifications()
    }
    pub async fn logout(self) -> Result<(), Error> {
        tracing::info!("Nostr Client Logging out");
        self.client.shutdown()?;
        Ok(())
    }
    pub async fn process_in(
        &self,
        keys: &Keys,
        input: NostrInput,
    ) -> Result<Option<NostrOutput>, Error> {
        let event_opt = match input {
            NostrInput::GetRelayStatusList => {
                let list = self.client.relay_status_list().await?;
                Some(BackendEvent::GotRelayStatusList(list).into())
            }
            NostrInput::RequestEventsOf((db_relay, contact_list)) => Some(
                request_events_of(&self.client, &keys, db_relay, contact_list)
                    .await?
                    .into(),
            ),
            NostrInput::GetContactListProfiles(_db_contacts) => {
                //     Some(
                //     get_contact_list_profile(&self.client, db_contacts)
                //         .await?
                //         .into(),
                // )
                None
            }

            NostrInput::SendEventToRelays(ns_event) => {
                Some(send_event_to_relays(&self.client, ns_event)?.into())
            }

            NostrInput::PrepareClient {
                relays,
                last_event,
                contact_list,
            } => Some(
                add_relays_and_connect(&self.client, &keys, &relays, last_event, contact_list)?
                    .into(),
            ),
            NostrInput::AddRelay(db_relay) => Some(add_relay(&self.client, db_relay)?.into()),
            NostrInput::DeleteRelay(db_relay) => Some(delete_relay(&self.client, db_relay)?.into()),
            NostrInput::ToggleRelayRead((db_relay, read)) => {
                Some(toggle_read_for_relay(&self.client, db_relay, read)?.into())
            }
            NostrInput::ToggleRelayWrite((db_relay, write)) => {
                Some(toggle_write_for_relay(&self.client, db_relay, write)?.into())
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

pub async fn create_channel(_client: &RelayPool) -> Result<BackendEvent, Error> {
    // tracing::debug!("create_channel");
    // let metadata = Metadata::new()
    //     .about("Channel about cars")
    //     .display_name("Best Cars")
    //     .name("Best Cars")
    //     .banner(Url::from_str("https://picsum.photos/seed/picsum/800/300")?)
    //     .picture(Url::from_str("https://picsum.photos/seed/picsum/200/300")?)
    //     .website(Url::from_str("https://picsum.photos/seed/picsum/200/300")?);

    // let event_id = client.new_channel(metadata).await?;

    // Ok(BackendEvent::ChannelCreated(event_id))
    todo!()
}

pub fn add_relay(client: &RelayPool, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("Adding relay to client: {}", db_relay.url);
    let opts = ns_client::RelayOptions::new(db_relay.read, db_relay.write);
    client.add_relay_with_opts(db_relay.url.as_str(), opts)?;
    Ok(BackEndInput::AddRelayToDb(db_relay))
}

pub fn add_relays_and_connect(
    client: &RelayPool,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
    contact_list: Vec<DbContact>,
) -> Result<BackEndInput, Error> {
    tracing::debug!("add_relays_and_connect");
    // Received from BackEndEvent::PrepareClient
    tracing::debug!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        let opts = ns_client::RelayOptions::new(r.read, r.write);
        match client.add_relay_with_opts(&r.url.to_string(), opts) {
            Ok(_) => tracing::debug!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    // tracing::debug!("Connecting to relays");
    // client.connect().await;

    tracing::info!("Subscribing to events");
    let filters = make_nostr_filters(keys.public_key(), last_event, &contact_list);
    client.subscribe(filters)?;
    // Ok(BackendEvent::SubscribedToEvents)

    Ok(BackEndInput::FinishedPreparingNostr)
}

pub fn toggle_read_for_relay(
    client: &RelayPool,
    db_relay: DbRelay,
    read: bool,
) -> Result<BackEndInput, Error> {
    tracing::debug!("toggle_read_for_relay");
    client.toggle_read_for(&db_relay.url, read)?;
    Ok(BackEndInput::ToggleRelayRead((db_relay, read)))
}
pub fn toggle_write_for_relay(
    client: &RelayPool,
    db_relay: DbRelay,
    write: bool,
) -> Result<BackEndInput, Error> {
    tracing::debug!("toggle_write_for_relay");
    client.toggle_write_for(&db_relay.url, write)?;
    Ok(BackEndInput::ToggleRelayWrite((db_relay, write)))
}

pub fn delete_relay(client: &RelayPool, db_relay: DbRelay) -> Result<BackEndInput, Error> {
    tracing::debug!("delete_relay");
    client.remove_relay(db_relay.url.as_str())?;
    Ok(BackEndInput::DeleteRelayFromDb(db_relay))
}

pub async fn request_events_of(
    _client: &RelayPool,
    _keys: &Keys,
    db_relay: DbRelay,
    contact_list: Vec<DbContact>,
) -> Result<BackendEvent, Error> {
    tracing::info!("Requesting events of {}", db_relay.url);
    tracing::debug!("contact_list: {}", contact_list.len());
    // if let Some(relay) = client.relays().await.get(&db_relay.url) {
    //     let filters = make_nostr_filters(keys.public_key().clone(), None, &contact_list);
    //     let timeout = Some(Duration::from_secs(10));
    //     relay.req_events_of(filters, timeout);
    // }
    Ok(BackendEvent::RequestedEventsOf(db_relay))
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

pub fn send_event_to_relays(
    client: &RelayPool,
    ns_event: nostr::Event,
) -> Result<BackendEvent, Error> {
    tracing::debug!("send_event_to_relays ID: {}", ns_event.id);
    tracing::debug!("{:?}", ns_event);
    let ns_event_id = ns_event.id;
    client.send_event(ns_event)?;
    Ok(BackendEvent::SentEventToRelays(ns_event_id))
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
