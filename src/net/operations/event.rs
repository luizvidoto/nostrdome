use crate::Error;
use nostr::{Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelayResponse},
    net::{nostr_events::NostrState, BackendEvent},
    types::ChatMessage,
};

pub async fn confirmed_event(
    ns_event: nostr::Event,
    relay_url: &Url,
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<Option<DbEvent>, Error> {
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    DbRelayResponse::insert_ok(pool, relay_url, &db_event).await?;

    if rows_changed == 0 {
        //maybe this never happens because the database checks
        // the event_hash uniqueness and sends an error
        tracing::debug!("Event already in database");
        return Ok(None);
    }

    Ok(Some(db_event))
}

pub async fn insert_other_kind(db_event: DbEvent) -> Result<Option<BackendEvent>, Error> {
    Ok(Some(BackendEvent::OtherKindEventInserted(db_event)))
}

async fn insert_pending(
    ns_event: nostr::Event,
    pool: &SqlitePool,
    nostr: &mut NostrState,
) -> Result<DbEvent, Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    pending_event = pending_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Err(Error::DuplicatedEvent);
    }

    nostr.client.send_event(ns_event).await?;

    Ok(pending_event)
}

pub async fn insert_pending_metadata(
    pool: &SqlitePool,
    ns_event: nostr::Event,
    _metadata: nostr::Metadata,
    nostr: &mut NostrState,
) -> Result<BackendEvent, Error> {
    let pending_event = insert_pending(ns_event, pool, nostr).await?;
    Ok(BackendEvent::PendingMetadata(pending_event))
}

pub async fn insert_pending_contact_list(
    pool: &SqlitePool,
    ns_event: nostr::Event,
    _contact_list: Vec<DbContact>,
    nostr: &mut NostrState,
) -> Result<BackendEvent, Error> {
    let pending_event = insert_pending(ns_event, pool, nostr).await?;
    Ok(BackendEvent::PendingContactList(pending_event))
}

pub async fn insert_pending_dm(
    pool: &SqlitePool,
    keys: &Keys,
    ns_event: nostr::Event,
    db_contact: DbContact,
    content: String,
    nostr: &mut NostrState,
) -> Result<BackendEvent, Error> {
    let pending_event = insert_pending(ns_event, pool, nostr).await?;
    let pending_msg = DbMessage::new(&pending_event, db_contact.pubkey())?;
    let row_id = DbMessage::insert_message(pool, &pending_msg).await?;
    let pending_msg = pending_msg.with_id(row_id);
    let chat_message =
        ChatMessage::from_db_message_content(keys, &pending_msg, &db_contact, &content)?;
    // let db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
    Ok(BackendEvent::PendingDM((db_contact, chat_message)))
}

pub async fn confirm_event(
    pool: &SqlitePool,
    event_hash: &nostr::EventId,
    relay_url: &Url,
) -> Result<DbEvent, Error> {
    let mut db_event = DbEvent::fetch_one(pool, event_hash)
        .await?
        .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
    if db_event.relay_url.is_none() {
        db_event = DbEvent::confirm_event(pool, relay_url, db_event).await?;
    }
    DbRelayResponse::insert_ok(pool, relay_url, &db_event).await?;
    Ok(db_event)
}
