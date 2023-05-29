use crate::Error;
use nostr::{secp256k1::XOnlyPublicKey, Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, TagInfo},
    net::BackendEvent,
    types::ChatMessage,
};

pub async fn handle_dm(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    db_event: &DbEvent,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("handle_dm");
    let info = TagInfo::from_db_event(db_event)?;
    let contact_pubkey = info.contact_pubkey(keys)?;

    // Parse the event and insert the message into the database
    let db_message = insert_message_from_event(db_event, pool, &contact_pubkey).await?;

    // If the message is from the user to themselves, log the error and return None
    if db_message.from_pubkey() == db_message.to_pubkey() {
        tracing::error!("Message is from the user to himself");
        return Ok(None);
    }

    // Determine the contact's public key and whether the message is from the user
    let contact_pubkey = determine_sender_receiver(&db_message, &keys.public_key());

    tracing::debug!("fetch_or_create_contact");
    let db_contact = match DbContact::fetch_one(pool, cache_pool, &contact_pubkey).await? {
        Some(db_contact) => db_contact,
        None => {
            tracing::debug!("Creating new contact with pubkey: {}", &contact_pubkey);
            let db_contact = DbContact::new(&contact_pubkey);
            DbContact::upsert_contact(pool, &db_contact).await?;
            db_contact
        }
    };

    let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;

    Ok(Some(BackendEvent::ReceivedDM {
        chat_message,
        db_contact,
        relay_url: relay_url.to_owned(),
    }))
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
) -> XOnlyPublicKey {
    tracing::debug!("determine_sender_receiver");
    if db_message.im_author(user_pubkey) {
        tracing::debug!("Message is from the user");
        db_message.to_pubkey()
    } else {
        tracing::debug!("Message is from contact");
        db_message.from_pubkey()
    }
}

pub async fn handle_dm_confirmation(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<BackendEvent, Error> {
    let relay_url = db_event
        .relay_url
        .as_ref()
        .ok_or(Error::NotConfirmedEvent(db_event.event_hash.to_owned()))?;

    if let Some(db_message) = DbMessage::fetch_by_event(pool, db_event.event_id()?).await? {
        let db_message = DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
        if let Some(db_contact) =
            DbContact::fetch_one(pool, cache_pool, &db_message.contact_chat()).await?
        {
            let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
            return Ok(BackendEvent::ConfirmedDM((db_contact, chat_message)));
        } else {
            tracing::error!("No contact found for confirmed message");
            return Err(Error::ContactNotFound(db_message.contact_chat().to_owned()));
        }
    }

    tracing::error!("No message found for confirmation event");
    Err(Error::EventWithoutMessage(db_event.event_id()?))
}
