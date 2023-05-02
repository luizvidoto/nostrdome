use crate::db::{DbChat, DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, MessageStatus};
use crate::error::Error;
use crate::net::events::backend::BackEndEvent;
use crate::types::ChatMessage;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Client, Contact, Filter, Keys, RelayMessage, Timestamp};
use nostr_sdk::{EventBuilder, Metadata};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

use super::events::Event;

pub async fn insert_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactInsert);
    }
    DbContact::insert(pool, &db_contact).await?;
    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn update_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update(pool, &db_contact).await?;
    Ok(Event::ContactUpdated(db_contact.clone()))
}

pub async fn delete_contact(pool: &SqlitePool, db_contact: &DbContact) -> Result<Event, Error> {
    DbContact::delete(pool, &db_contact).await?;
    Ok(Event::ContactDeleted(db_contact.clone()))
}

pub async fn import_contacts(
    keys: &Keys,
    pool: &SqlitePool,
    db_contacts: &[DbContact],
) -> Result<Event, Error> {
    for db_contact in db_contacts {
        if let Err(e) = insert_contact(keys, pool, db_contact).await {
            tracing::error!("{}", e);
        }
    }
    Ok(Event::ContactsImported(db_contacts.to_vec()))
}

pub async fn db_add_relay(pool: &SqlitePool, db_relay: DbRelay) -> Result<Event, Error> {
    DbRelay::insert(pool, &db_relay).await?;
    Ok(Event::RelayCreated(db_relay))
}

pub async fn received_dm(
    pool: &SqlitePool,
    keys: &Keys,
    db_message: DbMessage,
) -> Result<Event, Error> {
    tracing::info!("Inserting external message");
    // Insert message into the database and get the message ID
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    let db_message = db_message.with_id(msg_id);

    // Decrypt the message content
    let content = db_message.decrypt_message(keys)?;

    // Determine if the message is from the user or received from another user
    let (contact_pubkey, is_from_user) = if db_message.im_author(&keys.public_key()) {
        (db_message.to_pubkey(), true)
    } else {
        (db_message.from_pubkey(), false)
    };

    // Fetch the associated contact from the database
    match DbContact::fetch_one(pool, &contact_pubkey).await? {
        Some(mut db_contact) => {
            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;
            Ok(Event::ReceivedDM((db_contact, chat_message)))
        }
        None => {
            // Create a new contact and insert it into the database
            let mut db_contact = DbContact::new(&contact_pubkey);
            insert_contact(keys, pool, &db_contact).await?;

            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;

            Ok(Event::NewDMAndContact((db_contact, chat_message)))
        }
    }
}

pub async fn relay_response_ok(
    pool: &SqlitePool,
    db_event: &DbEvent,
    relay_url: &Url,
) -> Result<Event, Error> {
    let mut relay_response = DbRelayResponse::from_response(
        true,
        db_event.event_id()?,
        &db_event.event_hash,
        relay_url,
        "",
    );
    let id = DbRelayResponse::insert(pool, &relay_response).await?;
    relay_response = relay_response.with_id(id);
    Ok(Event::UpdateWithRelayResponse {
        relay_response,
        db_event: db_event.clone(),
        db_message: None,
    })
}

// TODO: remove this
pub async fn received_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    let event = insert_event(pool, keys, event, relay_url).await?;
    Ok(event)
}

pub async fn insert_specific_kind(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: Option<&Url>,
    db_event: DbEvent,
) -> Result<Event, Error> {
    let event = match db_event.kind {
        nostr_sdk::Kind::EncryptedDirectMessage => {
            // Convert DbEvent to DbMessage
            let db_message = DbMessage::from_db_event(db_event, relay_url)?;
            received_dm(pool, keys, db_message).await?
        }
        nostr_sdk::Kind::RecommendRelay => {
            println!("--- RecommendRelay ---");
            dbg!(&db_event);
            Event::EventInserted(db_event)
        }
        nostr_sdk::Kind::ContactList => {
            if db_event.pubkey == keys.public_key() {
                println!("--- My ContactList ---");
                let db_contacts: Vec<_> = db_event
                    .tags
                    .iter()
                    .filter_map(|t| DbContact::from_tag(t).ok())
                    .collect();
                import_contacts(keys, pool, &db_contacts).await?
            } else {
                println!("*** Others ContactList That Im in ***");
                Event::EventInserted(db_event)
            }
        }
        nostr_sdk::Kind::ChannelCreation => {
            // println!("--- ChannelCreation ---");
            // dbg!(db_event);
            Event::EventInserted(db_event)
        }
        nostr_sdk::Kind::ChannelMetadata => {
            // println!("--- ChannelMetadata ---");
            // dbg!(db_event);
            Event::EventInserted(db_event)
        }
        nostr_sdk::Kind::ChannelMessage => {
            // println!("--- ChannelMessage ---");
            // dbg!(db_event);
            Event::EventInserted(db_event)
        }
        nostr_sdk::Kind::ChannelHideMessage => {
            // println!("--- ChannelHideMessage ---");
            // dbg!(db_event);
            Event::EventInserted(db_event)
        }
        nostr_sdk::Kind::ChannelMuteUser => {
            // println!("--- ChannelMuteUser ---");
            // dbg!(db_event);
            Event::EventInserted(db_event)
        }
        // Kind::EventDeletion => todo!(),
        // Kind::PublicChatReserved45 => todo!(),
        // Kind::PublicChatReserved46 => todo!(),
        // Kind::PublicChatReserved47 => todo!(),
        // Kind::PublicChatReserved48 => todo!(),
        // Kind::PublicChatReserved49 => todo!(),
        // Kind::ZapRequest => todo!(),
        // Kind::Zap => todo!(),
        // Kind::MuteList => todo!(),
        // Kind::PinList => todo!(),
        // Kind::RelayList => todo!(),
        // Kind::Authentication => todo!(),
        _other => Event::EventInserted(db_event),
    };

    Ok(event)
}

pub async fn handle_insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: Option<&Url>,
    is_pending: bool,
) -> Result<Event, Error> {
    tracing::info!(
        "Inserting {} event: {:?}",
        if is_pending { "pending" } else { "confirmed" },
        event
    );

    let mut db_event = if is_pending {
        DbEvent::pending_event(event)?
    } else {
        DbEvent::confirmed_event(event)?
    };

    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    if let Some(url) = relay_url {
        let _ev = relay_response_ok(pool, &db_event, url).await?;
    }

    if rows_changed == 0 {
        return Ok(Event::None);
    }

    match insert_specific_kind(pool, keys, relay_url, db_event).await? {
        Event::EventInserted(db_event) => {
            if is_pending {
                Ok(Event::LocalPendingEvent(db_event))
            } else {
                Ok(Event::EventInserted(db_event))
            }
        }
        other => Ok(other),
    }
}

pub async fn insert_pending_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
) -> Result<Event, Error> {
    handle_insert_event(pool, keys, event, None, true).await
}

pub async fn insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    handle_insert_event(pool, keys, event, Some(relay_url), false).await
}

pub async fn prepare_client(pool: &SqlitePool) -> Result<Event, Error> {
    tracing::warn!("Preparing client");
    let relays = DbRelay::fetch(pool, None).await?;
    let last_event = DbEvent::fetch_last(pool).await?;

    Ok(Event::BackEndEvent(BackEndEvent::PrepareClient {
        relays,
        last_event,
    }))
}

pub async fn on_relay_message(
    pool: &SqlitePool,
    relay_url: &Url,
    relay_message: &RelayMessage,
) -> Result<Event, Error> {
    let event = match relay_message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message,
        } => {
            let mut db_event = DbEvent::fetch_one(pool, event_hash)
                .await?
                .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
            let mut db_message = None;

            if !db_event.confirmed {
                db_event = DbEvent::confirm_event(pool, db_event).await?;

                if let nostr_sdk::Kind::EncryptedDirectMessage = db_event.kind {
                    db_message = if let Some(db_message) =
                        DbMessage::fetch_one(pool, db_event.event_id()?).await?
                    {
                        let confirmed_db_message =
                            DbMessage::confirm_message(pool, db_message).await?;
                        Some(confirmed_db_message)
                    } else {
                        None
                    };
                }
            }

            let mut relay_response = DbRelayResponse::from_response(
                *status,
                db_event.event_id()?,
                event_hash,
                relay_url,
                message,
            );
            let id = DbRelayResponse::insert(pool, &relay_response).await?;
            relay_response = relay_response.with_id(id);
            Event::UpdateWithRelayResponse {
                relay_response,
                db_event,
                db_message,
            }
        }
        _ => Event::None,
    };

    Ok(event)
}

pub async fn _fetch_contacts_from_relays(nostr_client: &Client) -> Result<Vec<Contact>, Error> {
    let contacts = nostr_client
        .get_contact_list(Some(Duration::from_secs(10)))
        .await?;
    Ok(contacts)
}

pub async fn add_to_unseen_count(
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    db_contact = DbContact::add_to_unseen_count(pool, &mut db_contact).await?;
    Ok(Event::ContactUpdated(db_contact))
}

pub async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    tracing::info!("Fetching chat messages");
    let own_pubkey = keys.public_key();
    let chat = DbChat::new(&own_pubkey, db_contact.pubkey());
    let mut db_messages = chat.fetch_chat(pool).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for m in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        m.update_status(MessageStatus::Seen);
        DbMessage::update_message_status(pool, m).await?;
    }

    tracing::info!("Decrypting messages");
    for m in &mut db_messages {
        let content = m.decrypt_message(keys)?;
        let is_from_user = m.im_author(&keys.public_key());
        let chat_message = ChatMessage::from_db_message(&m, is_from_user, &db_contact, &content)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok(Event::GotChatMessages((db_contact, chat_messages)))
}

pub async fn fetch_relays_responses(pool: &SqlitePool, event_id: i64) -> Result<Event, Error> {
    let responses = DbRelayResponse::fetch_by_event(pool, event_id).await?;
    Ok(Event::GotRelayResponses(responses))
}

async fn _send_contact_list(client: &Client, list: &[DbContact]) -> Result<Event, Error> {
    let c_list: Vec<_> = list.iter().map(|c| c.into()).collect();
    let _event_id = client.set_contact_list(c_list).await?;

    Ok(Event::None)
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
) -> Result<Event, Error> {
    // let list = DbContact::fetch(pool).await?;
    let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();

    let builder = EventBuilder::set_contact_list(c_list);
    let event = builder.to_event(keys)?;

    let _event_id = client.send_event_to(url.to_owned(), event.clone()).await?;

    Ok(Event::BackEndEvent(BackEndEvent::InsertPendingEvent(event)))
}

pub async fn send_dm(
    nostr_client: &Client,
    keys: &Keys,
    db_contact: &DbContact,
    content: &str,
) -> Result<Event, Error> {
    tracing::info!("Sending DM to relays");
    let mut has_event: Option<(nostr_sdk::Event, Url)> = None;
    let builder =
        EventBuilder::new_encrypted_direct_msg(&keys, db_contact.pubkey().to_owned(), content)?;
    let event = builder.to_event(keys)?;

    for (url, relay) in nostr_client.relays().await {
        if !relay.opts().write() {
            // return Err(Error::WriteActionsDisabled(url.clone()))
            tracing::error!("{}", Error::WriteActionsDisabled(url.to_string()));
            continue;
        }

        if let Ok(_id) = nostr_client.send_event_to(url.clone(), event.clone()).await {
            has_event = Some((event.clone(), url.clone()));
        }
    }

    if let Some((event, _relay_url)) = has_event {
        Ok(Event::BackEndEvent(BackEndEvent::InsertPendingEvent(event)))
    } else {
        Err(Error::NoRelayToWrite)
    }
}

pub async fn add_relay(nostr_client: &Client, db_relay: DbRelay) -> Result<Event, Error> {
    tracing::info!("Adding relay to client: {}", db_relay.url);
    nostr_client.add_relay(db_relay.url.as_str(), None).await?;
    Ok(Event::backend(BackEndEvent::AddRelayToDb(db_relay)))
}

pub async fn connect_relay(
    nostr_client: &Client,
    keys: &Keys,
    db_relay: DbRelay,
    last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    let event = if let Some(relay) = nostr_client
        .relays()
        .await
        .values()
        .find(|r| &r.url() == &db_relay.url)
    {
        if let nostr_sdk::RelayStatus::Connected = relay.status().await {
            tracing::info!("Disconnecting from relay");
            nostr_client.disconnect_relay(db_relay.url.as_str()).await?;
            // it was bugged without sleep
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        tracing::info!("Connecting to relay: {}", db_relay.url);
        nostr_client.connect_relay(db_relay.url.as_str()).await?;

        request_events(nostr_client, &keys.public_key(), last_event).await?;

        Event::RelayConnected(db_relay)
    } else {
        tracing::warn!("Relay not found on client: {}", db_relay.url);
        add_relay(nostr_client, db_relay).await?
    };

    Ok(event)
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
    nostr_client: &Client,
    keys: &Keys,
    relays: &[DbRelay],
    last_event: Option<DbEvent>,
) -> Result<Event, Error> {
    // Received from BackEndEvent::PrepareClient
    tracing::warn!("Adding relays to client");
    for r in relays {
        match nostr_client.add_relay(&r.url.to_string(), None).await {
            Ok(_) => tracing::warn!("Nostr Client Added Relay: {}", &r.url),
            Err(e) => tracing::error!("{}", e),
        }
    }

    tracing::info!("Connecting to relays");
    nostr_client.connect().await;

    request_events(&nostr_client, &keys.public_key(), last_event).await?;

    Ok(Event::FinishedPreparing)
}

pub async fn fetch_contacts(pool: &SqlitePool) -> Result<Event, Error> {
    let contacts = DbContact::fetch(pool).await?;
    Ok(Event::GotContacts(contacts))
}
pub async fn delete_relay(client: &Client, db_relay: DbRelay) -> Result<Event, Error> {
    client.remove_relay(db_relay.url.as_str()).await?;
    Ok(Event::backend(BackEndEvent::DeleteRelayFromDb(db_relay)))
}
pub async fn db_delete_relay(pool: &SqlitePool, db_relay: DbRelay) -> Result<Event, Error> {
    DbRelay::delete(pool, &db_relay).await?;
    Ok(Event::RelayDeleted(db_relay))
}
pub async fn fetch_relays(pool: &SqlitePool) -> Result<Event, Error> {
    let relays = DbRelay::fetch(pool, None).await?;
    Ok(Event::GotRelays(relays))
}

pub async fn request_events(
    nostr_client: &Client,
    public_key: &XOnlyPublicKey,
    last_event: Option<DbEvent>,
) -> Result<(), Error> {
    tracing::info!("Requesting events");

    // if has last_event, request events since last_event.timestamp
    // else request events since 0
    let last_timestamp: u64 = last_event
        .map(|e| e.created_at.timestamp_millis() as u64)
        .unwrap_or(0);

    let sent_msgs_sub_past = Filter::new()
        .author(public_key.to_string())
        .since(Timestamp::from(last_timestamp))
        .until(Timestamp::now());
    let recv_msgs_sub_past = Filter::new()
        .pubkey(public_key.to_owned())
        .since(Timestamp::from(last_timestamp))
        .until(Timestamp::now());

    let timeout = Duration::from_secs(10);
    nostr_client
        .req_events_of(vec![sent_msgs_sub_past, recv_msgs_sub_past], Some(timeout))
        .await;

    let sent_msgs_sub_future = Filter::new()
        .author(public_key.to_string())
        .since(Timestamp::now());
    let recv_msgs_sub_future = Filter::new()
        .pubkey(public_key.to_owned())
        .since(Timestamp::now());

    nostr_client
        .subscribe(vec![sent_msgs_sub_future, recv_msgs_sub_future])
        .await;

    Ok(())
}

pub async fn toggle_read_for_relay(
    nostr_client: &Client,
    db_relay: DbRelay,
    read: bool,
) -> Result<Event, Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_read(read)
    }
    Ok(Event::BackEndEvent(BackEndEvent::ToggleRelayRead((
        db_relay, read,
    ))))
}
pub async fn toggle_write_for_relay(
    nostr_client: &Client,
    db_relay: DbRelay,
    write: bool,
) -> Result<Event, Error> {
    let mut relays = nostr_client.relays().await;
    if let Some(relay) = relays.get_mut(&db_relay.url) {
        relay.opts().set_write(write)
    }
    Ok(Event::BackEndEvent(BackEndEvent::ToggleRelayWrite((
        db_relay, write,
    ))))
}
