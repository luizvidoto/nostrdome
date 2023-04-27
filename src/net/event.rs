use crate::db::{store_last_event_timestamp, DbContact, DbEvent, DbMessage, DbRelayResponse};
use crate::error::Error;
use crate::net::contact::insert_contact;
use crate::types::ChatMessage;

use nostr_sdk::{Client, Contact, EventBuilder, Keys, Kind, Url};
use sqlx::SqlitePool;

use super::{Event, SuccessKind};

pub async fn received_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    let event = insert_event(pool, keys, event, relay_url).await?;

    store_last_event_timestamp(pool).await?;

    Ok(event)
}

async fn received_encrypted_dm(
    pool: &SqlitePool,
    keys: &Keys,
    db_event: DbEvent,
    relay_url: Option<&Url>,
) -> Result<SuccessKind, Error> {
    // Convert DbEvent to DbMessage
    let db_message = DbMessage::from_db_event(db_event, relay_url)?;
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
            Ok(SuccessKind::ReceivedDM((db_contact, chat_message)))
        }
        None => {
            // Create a new contact and insert it into the database
            let mut db_contact = DbContact::new(&contact_pubkey);
            insert_contact(keys, pool, &db_contact).await?;

            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;

            Ok(SuccessKind::NewDMAndContact((db_contact, chat_message)))
        }
    }
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
        // Ok(insert_pending_event(pool, keys, event).await?)
        Ok(Event::InsertPendingEvent(event))
    } else {
        Err(Error::NoRelayToWrite)
    }
}

async fn relay_response_ok(
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
    Ok(Event::DBSuccessEvent(
        SuccessKind::UpdateWithRelayResponse {
            relay_response,
            db_event: db_event.clone(),
            db_message: None,
        },
    ))
}
async fn insert_specific_kind(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: Option<&Url>,
    db_event: &DbEvent,
) -> Result<Option<Event>, Error> {
    let event = match db_event.kind {
        Kind::EncryptedDirectMessage => {
            let database_success_event_kind =
                received_encrypted_dm(pool, keys, db_event.clone(), relay_url).await?;
            Some(Event::DBSuccessEvent(database_success_event_kind))
        }
        Kind::RecommendRelay => {
            println!("--- RecommendRelay ---");
            dbg!(db_event);
            None
        }
        Kind::ContactList => {
            println!("--- ContactList ---");
            dbg!(db_event);
            None
        }
        Kind::ChannelCreation => {
            // println!("--- ChannelCreation ---");
            // dbg!(db_event);
            None
        }
        Kind::ChannelMetadata => {
            // println!("--- ChannelMetadata ---");
            // dbg!(db_event);
            None
        }
        Kind::ChannelMessage => {
            // println!("--- ChannelMessage ---");
            // dbg!(db_event);
            None
        }
        Kind::ChannelHideMessage => {
            // println!("--- ChannelHideMessage ---");
            // dbg!(db_event);
            None
        }
        Kind::ChannelMuteUser => {
            // println!("--- ChannelMuteUser ---");
            // dbg!(db_event);
            None
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
        _ => None,
    };

    Ok(event)
}

async fn handle_insert_event(
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

    match insert_specific_kind(pool, keys, relay_url, &db_event).await? {
        Some(has_event) => Ok(has_event),
        None => {
            if is_pending {
                Ok(Event::LocalPendingEvent(db_event))
            } else {
                Ok(Event::DBSuccessEvent(SuccessKind::EventInserted(db_event)))
            }
        }
    }
}

pub async fn insert_pending_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
) -> Result<Event, Error> {
    handle_insert_event(pool, keys, event, None, true).await
}

async fn insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    handle_insert_event(pool, keys, event, Some(relay_url), false).await
}

pub async fn send_contact_list_to(
    pool: &SqlitePool,
    keys: &Keys,
    client: &Client,
    url: Url,
) -> Result<Event, Error> {
    let list = DbContact::fetch(pool).await?;
    let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();

    let builder = EventBuilder::set_contact_list(c_list);
    let event = builder.to_event(keys)?;

    let _event_id = client.send_event_to(url, event.clone()).await?;

    Ok(insert_pending_event(pool, keys, event).await?)
}
