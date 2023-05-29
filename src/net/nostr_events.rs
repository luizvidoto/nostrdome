use crate::Error;

use nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::{RelayOptions, RelayPoolNotification};
use tokio::sync::broadcast;

use crate::db::{DbContact, DbEvent, DbRelay};
use crate::net::BackendEvent;
use nostr::{Filter, Keys, Kind, SubscriptionId, Timestamp};

use super::backend::BackEndInput;

pub struct NostrState {
    pub keys: Keys,
    pub client: nostr_sdk::Client,
    pub notifications: broadcast::Receiver<RelayPoolNotification>,
}
impl NostrState {
    pub async fn new(keys: &Keys) -> Self {
        let client = nostr_sdk::Client::new(keys);
        let notifications = client.notifications();
        Self {
            keys: keys.to_owned(),
            client,
            notifications,
        }
    }
    pub async fn logout(self) -> Result<(), Error> {
        self.client.shutdown().await?;
        Ok(())
    }
}

pub async fn create_channel(_client: &nostr_sdk::Client) -> Result<BackendEvent, Error> {
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

pub async fn add_relays_and_connect(
    client: &nostr_sdk::Client,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
) -> Result<BackendEvent, Error> {
    tracing::info!("Adding relays to client: {}", relays.len());

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

    client.connect().await;

    let last_timestamp_secs: u64 = last_event
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.local_creation - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0);
    tracing::info!("last event timestamp: {}", last_timestamp_secs);

    let contact_list_sub_id = SubscriptionId::new(SubscriptionType::ContactList.to_string());
    client
        .req_events_of(
            vec![contact_list_filter(keys.public_key(), last_timestamp_secs)],
            None,
        )
        .await;
    let user_metadata_id = SubscriptionId::new(SubscriptionType::UserMetadata.to_string());
    client
        .req_events_of(vec![user_metadata(keys.public_key())], None)
        .await;
    let messages_sub_id = SubscriptionId::new(SubscriptionType::Messages.to_string());
    client
        .subscribe(messages_filter(keys.public_key(), last_timestamp_secs))
        .await;
    // client.subscribe_eose(id, channel_filter)?;

    Ok(BackendEvent::FinishedPreparing)
}
fn channel_search_filter() -> Vec<Filter> {
    tracing::trace!("channel_search_filter");

    let channel_filter = Filter::new()
        .kinds(vec![Kind::ChannelCreation, Kind::ChannelMetadata])
        .limit(CHANNEL_SEARCH_LIMIT);

    vec![channel_filter]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    ContactList,
    ContactListMetadata,
    UserMetadata,
    Messages,
    Channel,
    ChannelMetadata,
}
impl SubscriptionType {
    pub const ALL: [SubscriptionType; 6] = [
        SubscriptionType::ContactList,
        SubscriptionType::ContactListMetadata,
        SubscriptionType::UserMetadata,
        SubscriptionType::Messages,
        SubscriptionType::Channel,
        SubscriptionType::ChannelMetadata,
    ];
}
impl std::fmt::Display for SubscriptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionType::ContactList => write!(f, "ContactList"),
            SubscriptionType::ContactListMetadata => write!(f, "ContactListMetadata"),
            SubscriptionType::UserMetadata => write!(f, "UserMetadata"),
            SubscriptionType::Messages => write!(f, "Messages"),
            SubscriptionType::Channel => write!(f, "Channel"),
            SubscriptionType::ChannelMetadata => write!(f, "ChannelMetadata"),
        }
    }
}
pub fn contact_list_metadata(contact_list: &[DbContact]) -> Filter {
    let all_pubkeys = contact_list
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();
    Filter::new().authors(all_pubkeys).kind(Kind::Metadata)
    // .since(Timestamp::from(last_timestamp_secs))
}
pub fn user_metadata(pubkey: XOnlyPublicKey) -> Filter {
    Filter::new()
        .author(pubkey.to_string())
        .kind(Kind::Metadata)
}

fn contact_list_filter(public_key: XOnlyPublicKey, last_timestamp_secs: u64) -> Filter {
    let user_contact_list = Filter::new()
        .author(public_key.to_string())
        .kind(Kind::ContactList)
        .since(Timestamp::from(last_timestamp_secs));
    user_contact_list
}

fn messages_filter(public_key: XOnlyPublicKey, last_timestamp_secs: u64) -> Vec<Filter> {
    let sent_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .author(public_key.to_string())
        .since(Timestamp::from(last_timestamp_secs));
    let recv_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .pubkey(public_key)
        .since(Timestamp::from(last_timestamp_secs));
    vec![sent_msgs, recv_msgs]
}

// pub fn toggle_read_for_relay(
//     client: &RelayPool,
//     db_relay: DbRelay,
//     read: bool,
// ) -> Result<BackEndInput, Error> {
//     tracing::debug!("toggle_read_for_relay");
//     client.toggle_read_for(&db_relay.url, read)?;
//     Ok(BackEndInput::ToggleRelayRead((db_relay, read)))
// }
// pub fn toggle_write_for_relay(
//     client: &RelayPool,
//     db_relay: DbRelay,
//     write: bool,
// ) -> Result<BackEndInput, Error> {
//     tracing::debug!("toggle_write_for_relay");
//     client.toggle_write_for(&db_relay.url, write)?;
//     Ok(BackEndInput::ToggleRelayWrite((db_relay, write)))
// }

// pub fn delete_relay(client: &RelayPool, db_relay: DbRelay) -> Result<BackEndInput, Error> {
//     tracing::debug!("delete_relay");
//     client.remove_relay(db_relay.url.as_str())?;
//     Ok(BackEndInput::DeleteRelayFromDb(db_relay))
// }

const CHANNEL_SEARCH_LIMIT: usize = 100;
