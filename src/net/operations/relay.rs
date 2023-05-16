use std::str::FromStr;

use nostr_sdk::{Client, Keys, Metadata, Url};

use crate::{
    db::{DbContact, DbEvent, DbRelay},
    error::Error,
    net::{
        events::{backend::BackEndInput, Event},
        operations::subscribe::subscribe_to_events,
    },
};

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
    tracing::debug!("Adding relay to client: {}", db_relay.url);
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
    tracing::debug!("Adding relays to client: {}", relays.len());

    // Only adds to the HashMap
    for r in relays {
        match client.add_relay(&r.url.to_string(), None).await {
            Ok(_) => tracing::debug!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::debug!("Connecting to relays");
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
