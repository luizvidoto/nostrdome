use std::time::Duration;

use nostr_sdk::{secp256k1::XOnlyPublicKey, Client, Filter, Keys, Kind, Timestamp};

use crate::{
    db::{DbContact, DbEvent, DbRelay},
    error::Error,
    net::events::Event,
};

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

pub async fn send_event_to_relays(
    client: &Client,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::info!("send_event_to_relays: {}", ns_event.id);
    tracing::debug!("{:?}", ns_event);
    let event_hash = client.send_event(ns_event).await?;
    Ok(Event::SentEventToRelays(event_hash))
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
