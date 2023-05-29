use crate::Error;
use nostr::Keys;
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbMessage},
    net::BackendEvent,
    types::ChatMessage,
};

pub async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: DbContact,
) -> Result<BackendEvent, Error> {
    tracing::debug!("Fetching chat messages");
    let mut db_messages = DbMessage::fetch_chat(pool, db_contact.pubkey()).await?;
    let mut chat_messages = vec![];

    tracing::debug!("Updating unseen messages to marked as seen");
    for db_message in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        DbMessage::message_seen(pool, db_message).await?;
    }

    tracing::debug!("Decrypting messages");
    for db_message in &db_messages {
        let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
        chat_messages.push(chat_message);
    }

    Ok(BackendEvent::GotChatMessages((db_contact, chat_messages)))
}

pub async fn insert_contact_from_event(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("Inserting contact from event");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(None);
    }

    // Check if the contact is already in the database
    if DbContact::have_contact(pool, &db_contact.pubkey()).await? {
        return update_contact_basic(keys, pool, db_contact).await;
    }

    // If the contact is not in the database, insert it
    DbContact::insert(pool, &db_contact).await?;

    Ok(Some(BackendEvent::ContactCreated(db_contact.clone())))
}

pub async fn update_contact_basic(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("update_contact_basic");
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update_basic(pool, &db_contact).await?;
    Ok(Some(BackendEvent::ContactUpdated(db_contact.clone())))
}

pub async fn update_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("update_contact");
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update(pool, &db_contact).await?;
    Ok(Some(BackendEvent::ContactUpdated(db_contact.clone())))
}

pub async fn add_new_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("add_new_contact");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(None);
    }

    DbContact::insert(pool, &db_contact).await?;

    Ok(Some(BackendEvent::ContactCreated(db_contact.clone())))
}

pub async fn delete_contact(
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Option<BackendEvent>, Error> {
    tracing::debug!("delete_contact");
    DbContact::delete(pool, &db_contact).await?;
    Ok(Some(BackendEvent::ContactDeleted(db_contact.clone())))
}
