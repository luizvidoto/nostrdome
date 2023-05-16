use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbEvent, DbMessage, TagInfo},
    error::Error,
    net::{events::Event, operations::contact::fetch_or_create_contact},
};

pub async fn handle_dm(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("handle_dm");
    // create event struct
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    // insert into database
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::info!("Event already in database");
        return Ok(Event::None);
    }

    let info = TagInfo::from_db_event(&db_event)?;
    let contact_pubkey = info.contact_pubkey(keys)?;

    // Parse the event and insert the message into the database
    let db_message = insert_message_from_event(&db_event, pool, &contact_pubkey).await?;

    // If the message is from the user to themselves, log the error and return None
    if db_message.from_pubkey() == db_message.to_pubkey() {
        tracing::error!("Message is from the user to himself");
        return Ok(Event::None);
    }

    // Determine the contact's public key and whether the message is from the user
    let (contact_pubkey, _is_from_user) =
        determine_sender_receiver(&db_message, &keys.public_key());

    // Fetch or create the associated contact, update the contact's message, and return the event
    let event = fetch_or_create_contact(
        pool,
        cache_pool,
        keys,
        relay_url,
        &contact_pubkey,
        &db_message,
    )
    .await?;

    Ok(event)
}

async fn insert_message_from_event(
    db_event: &DbEvent,
    pool: &SqlitePool,
    contact_pubkey: &XOnlyPublicKey,
) -> Result<DbMessage, Error> {
    tracing::debug!("insert_message_from_event");
    let db_message = DbMessage::confirmed_message(db_event, contact_pubkey)?;
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    Ok(db_message.with_id(msg_id))
}

fn determine_sender_receiver(
    db_message: &DbMessage,
    user_pubkey: &XOnlyPublicKey,
) -> (XOnlyPublicKey, bool) {
    tracing::debug!("determine_sender_receiver");
    if db_message.im_author(user_pubkey) {
        tracing::debug!("Message is from the user");
        (db_message.to_pubkey(), true)
    } else {
        tracing::debug!("Message is from contact");
        (db_message.from_pubkey(), false)
    }
}
