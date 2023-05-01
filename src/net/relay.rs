use crate::db::{DbEvent, DbRelay};
use crate::error::Error;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Client, Filter, Keys, Timestamp, Url};
use std::time::Duration;

pub async fn add_relay(nostr_client: &Client, url: &Url) -> Result<(), Error> {
    tracing::info!("Add relay to client: {}", url);
    nostr_client.add_relay(url.as_str(), None).await?;
    Ok(())
}
pub async fn update_relay_db_and_client(_nostr_client: &Client, _url: &Url) -> Result<(), Error> {
    tracing::info!("Updating relay db");
    Ok(())
}

pub async fn connect_relay(nostr_client: &Client, relay_url: &Url) -> Result<(), Error> {
    if let Some(relay) = nostr_client
        .relays()
        .await
        .values()
        .find(|r| &r.url() == relay_url)
    {
        if let nostr_sdk::RelayStatus::Connected = relay.status().await {
            tracing::info!("Disconnecting from relay");
            nostr_client.disconnect_relay(relay_url.as_str()).await?;
            // it was bugged without sleep
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        tracing::info!("Connecting to relay: {}", relay_url);
        nostr_client.connect_relay(relay_url.as_str()).await?;
    } else {
        tracing::warn!("Relay not found on client: {}", relay_url);
        add_relay(nostr_client, relay_url).await?;
    }

    Ok(())
}

// Add relays to client
// for r in vec![
//     // "wss://eden.nostr.land",
//     // "wss://relay.snort.social",
//     // "wss://relay.nostr.band",
//     // "wss://nostr.fmt.wiz.biz",
//     // "wss://relay.damus.io",
//     // "wss://nostr.anchel.nl/",
//     // "ws://192.168.15.119:8080"
//     // "ws://192.168.15.151:8080",
//     "ws://0.0.0.0:8080",
// ] {
//     match nostr_client.add_relay(r, None).await {
//         Ok(_) => tracing::info!("Nostr Client Added Relay: {}", r),
//         Err(e) => tracing::error!("{}", e),
//     }
// }

pub async fn add_relays_and_connect(
    nostr_client: &Client,
    relays: &[DbRelay],
    keys: &Keys,
    last_event: Option<DbEvent>,
) -> Result<(), Error> {
    tracing::info!("Adding relays to client");
    for r in relays {
        match nostr_client.add_relay(&r.url.to_string(), None).await {
            Ok(_) => tracing::info!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::info!("Connecting to relays");
    nostr_client.connect().await;

    request_events(
        &nostr_client,
        &keys.public_key(),
        &keys.public_key(),
        last_event,
    )
    .await?;

    Ok(())
}
pub async fn toggle_read_for_relay(
    nostr_client: &Client,
    url: &Url,
    read: bool,
) -> Result<(), Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(url) {
        relay.opts().set_read(read)
    }
    Ok(())
}
pub async fn toggle_write_for_relay(
    nostr_client: &Client,
    url: &Url,
    write: bool,
) -> Result<(), Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(url) {
        relay.opts().set_write(write)
    }
    Ok(())
}

pub async fn request_events(
    nostr_client: &Client,
    own_pubkey: &XOnlyPublicKey,
    pubkey: &XOnlyPublicKey,
    last_event: Option<DbEvent>,
) -> Result<(), Error> {
    tracing::info!("Requesting events");

    // if has last_event, request events since last_event.timestamp
    // else request events since 0
    let last_timestamp: u64 = last_event
        .map(|e| e.created_at.timestamp_millis() as u64)
        .unwrap_or(0);

    let sent_msgs_sub_past = Filter::new()
        .author(own_pubkey.to_string())
        // .kind(Kind::EncryptedDirectMessage)
        .since(Timestamp::from(last_timestamp))
        .until(Timestamp::now());
    let recv_msgs_sub_past = Filter::new()
        .pubkey(pubkey.to_owned())
        // .kind(Kind::EncryptedDirectMessage)
        .since(Timestamp::from(last_timestamp))
        .until(Timestamp::now());

    let timeout = Duration::from_secs(10);
    nostr_client
        .req_events_of(vec![sent_msgs_sub_past, recv_msgs_sub_past], Some(timeout))
        .await;

    // let sent_msgs_sub_future = Filter::new()
    //     .author(own_pubkey.to_string())
    //     // .kind(Kind::EncryptedDirectMessage)
    //     .since(Timestamp::now());
    // let recv_msgs_sub_future = Filter::new()
    //     .pubkey(pubkey.to_owned())
    //     // .kind(Kind::EncryptedDirectMessage)
    //     .since(Timestamp::now());

    // nostr_client
    //     .subscribe(vec![sent_msgs_sub_future, recv_msgs_sub_future])
    //     .await;

    Ok(())
}
