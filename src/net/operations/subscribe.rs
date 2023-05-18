use std::time::Duration;

use futures::channel::mpsc;
use nostr_sdk::{
    secp256k1::XOnlyPublicKey, Client, ClientMessage, Filter, Keys, Kind, RelayStatus, Timestamp,
};

use crate::{
    db::{DbContact, DbEvent, DbRelay},
    error::Error,
    net::events::{nostr::NostrOutput, Event},
};

pub async fn request_events_of(
    client: &Client,
    keys: &Keys,
    db_relay: DbRelay,
    contact_list: Vec<DbContact>,
) -> Result<Event, Error> {
    tracing::debug!("Requesting events of {}", db_relay.url);
    tracing::debug!("contact_list: {}", contact_list.len());
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
    tracing::debug!("Subscribing to events");

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

pub async fn send_event_to_relays(
    client: &Client,
    channel: &mut mpsc::Sender<NostrOutput>,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("send_event_to_relays ID: {}", ns_event.id);
    tracing::debug!("{:?}", ns_event);
    for (url, relay) in client.relays().await {
        let status = relay.status().await;
        if let RelayStatus::Connected = status {
            let msg = ClientMessage::Event(Box::new(ns_event.clone()));
            tracing::debug!("To relay: {}", &url);
            match relay.send_msg(msg, false).await {
                Ok(_) => {
                    if let Err(e) = channel.try_send(NostrOutput::Ok(Event::SentEventTo((
                        url,
                        ns_event.id.clone(),
                    )))) {
                        tracing::error!("Error sending event to back: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("Error sending event to relay: {}", e);
                }
            }
        }
    }
    // let event_hash = client.send_event(ns_event).await?;
    Ok(Event::SentEventToRelays(ns_event.id))
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
