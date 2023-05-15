use std::time::Duration;

use nostr_sdk::{secp256k1::XOnlyPublicKey, Client, Filter, Keys, Kind, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbMessage},
    error::Error,
    net::events::Event,
    types::ChatMessage,
};

pub async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    tracing::info!("Fetching chat messages");
    let mut db_messages = DbMessage::fetch_chat(pool, db_contact.pubkey()).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for db_message in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        DbMessage::message_seen(pool, db_message).await?;
    }

    tracing::info!("Decrypting messages");
    for db_message in &db_messages {
        let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok(Event::GotChatMessages((db_contact, chat_messages)))
}

pub async fn get_contact_profile(client: &Client, db_contact: DbContact) -> Result<Event, Error> {
    tracing::debug!("get_contact_profile");
    let filter = Filter::new()
        .author(db_contact.pubkey().to_string())
        .kind(Kind::Metadata);
    let timeout = Some(Duration::from_secs(10));
    client.req_events_of(vec![filter], timeout).await;
    Ok(Event::RequestedContactProfile(db_contact))
}

pub async fn get_contact_list_profile(
    client: &Client,
    db_contacts: Vec<DbContact>,
) -> Result<Event, Error> {
    tracing::debug!("get_contact_list_profile: {}", db_contacts.len());
    let all_pubkeys = db_contacts
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();
    let filter = Filter::new().authors(all_pubkeys).kind(Kind::Metadata);
    let timeout = Some(Duration::from_secs(10));
    client.req_events_of(vec![filter], timeout).await;
    Ok(Event::RequestedContactListProfiles)
}

pub async fn insert_contact_from_event(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    tracing::debug!("Inserting contact from event");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(Event::None);
    }

    // Check if the contact is already in the database
    if DbContact::have_contact(pool, &db_contact.pubkey()).await? {
        return update_contact_basic(keys, pool, db_contact).await;
    }

    // If the contact is not in the database, insert it
    DbContact::insert(pool, &db_contact).await?;

    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn update_contact_basic(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    tracing::debug!("update_contact_basic");
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update_basic(pool, &db_contact).await?;
    Ok(Event::ContactUpdated(db_contact.clone()))
}

pub async fn update_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    tracing::debug!("update_contact");
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update(pool, &db_contact).await?;
    Ok(Event::ContactUpdated(db_contact.clone()))
}

pub async fn add_new_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    tracing::debug!("add_new_contact");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(Event::None);
    }

    DbContact::insert(pool, &db_contact).await?;

    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn delete_contact(pool: &SqlitePool, db_contact: &DbContact) -> Result<Event, Error> {
    tracing::debug!("delete_contact");
    DbContact::delete(pool, &db_contact).await?;
    Ok(Event::ContactDeleted(db_contact.clone()))
}

pub async fn fetch_or_create_contact(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    contact_pubkey: &XOnlyPublicKey,
    db_message: &DbMessage,
) -> Result<Event, Error> {
    tracing::debug!("fetch_or_create_contact");
    let mut db_contact = match DbContact::fetch_one(pool, contact_pubkey).await? {
        Some(db_contact) => db_contact,
        None => {
            tracing::info!("Creating new contact with pubkey: {}", contact_pubkey);
            let db_contact = DbContact::new(contact_pubkey);
            DbContact::insert(pool, &db_contact).await?;
            db_contact
        }
    };

    tracing::debug!("Update last message and contact in the database");
    let chat_message = ChatMessage::from_db_message(keys, db_message, &db_contact)?;
    db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
    Ok(Event::ReceivedDM {
        chat_message,
        db_contact,
        relay_url: relay_url.to_owned(),
    })
}

pub async fn import_contacts(
    keys: &Keys,
    pool: &SqlitePool,
    db_contacts: Vec<DbContact>,
    is_replace: bool,
) -> Result<Event, Error> {
    for db_contact in &db_contacts {
        if is_replace {
            DbContact::delete(pool, db_contact).await?;
            add_new_contact(keys, pool, db_contact).await?;
        } else {
            update_contact(keys, pool, db_contact).await?;
        }
    }
    Ok(Event::FileContactsImported(db_contacts))
}
