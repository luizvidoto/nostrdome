use crate::db::{store_last_event_received, DbContact, DbEvent, DbMessage};
use crate::error::Error;
use crate::net::contact::insert_contact;
use crate::types::ChatMessage;

use nostr_sdk::{Keys, Kind, Url};
use sqlx::SqlitePool;

use super::{DatabaseSuccessEventKind, Event};

pub async fn insert_event_with_tt(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    let event = insert_event(pool, keys, event, relay_url).await?;

    store_last_event_received(pool).await?;

    Ok(event)
}

async fn insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    event: nostr_sdk::Event,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::info!("Inserting event: {:?}", event);

    let mut db_event = DbEvent::from_event(event)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event.event_id = Some(row_id);

    if rows_changed == 0 {
        return Ok(Event::None);
    }

    if let Kind::EncryptedDirectMessage = db_event.kind {
        let database_success_event_kind =
            insert_encrypted_dm(pool, keys, db_event, relay_url).await?;
        return Ok(Event::DatabaseSuccessEvent(database_success_event_kind));
    }

    Ok(Event::DatabaseSuccessEvent(
        DatabaseSuccessEventKind::EventInserted(db_event),
    ))
}

async fn insert_encrypted_dm(
    pool: &SqlitePool,
    keys: &Keys,
    db_event: DbEvent,
    relay_url: &Url,
) -> Result<DatabaseSuccessEventKind, Error> {
    // Convert DbEvent to DbMessage
    let db_message = DbMessage::from_db_event(db_event, Some(relay_url.to_owned()))?;
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
            Ok(DatabaseSuccessEventKind::ReceivedDM((
                db_contact,
                chat_message,
            )))
        }
        None => {
            // Create a new contact and insert it into the database
            let mut db_contact = DbContact::new(&contact_pubkey);
            insert_contact(keys, pool, &db_contact).await?;

            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;

            Ok(DatabaseSuccessEventKind::NewDMAndContact((
                db_contact,
                chat_message,
            )))
        }
    }
}
