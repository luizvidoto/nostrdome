use crate::{
    db::{DbContact, DbRelayResponse},
    error::Error,
    utils::ns_event_to_millis,
};
use futures_util::SinkExt;
use nostr::{Keys, Kind};
use sqlx::SqlitePool;
use url::Url;

use crate::{db::DbEvent, net::BackendEvent};

pub async fn received_contact_list(
    pool: &SqlitePool,
    url: &Url,
    ns_event: &nostr::Event,
) -> Result<Option<DbEvent>, Error> {
    match DbEvent::fetch_last_kind(pool, Kind::ContactList).await? {
        Some(db_event) => {
            if db_event.event_hash == ns_event.id {
                // if event already in the database, just confirmed it
                tracing::info!("ContactList already in the database");
                DbRelayResponse::insert_ok(pool, url, &db_event).await?;
                return Ok(None);
            }
            // if event is older than the last one, ignore it
            if db_event.created_at.timestamp_millis() > (ns_event_to_millis(ns_event.created_at)) {
                tracing::info!("ContactList is older than the last one");
                return Ok(None);
            } else {
                // delete old and insert new contact list
                tracing::info!("ContactList is newer than the last one");
                DbEvent::delete(pool, db_event.event_id).await?;
                DbContact::delete_all(pool).await?;
            }
        }
        None => {
            tracing::info!("No ContactList in the database - inserting");
        }
    }
    let db_event = DbEvent::insert(pool, url, ns_event).await?;

    Ok(db_event)
}

pub async fn handle_contact_list(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    pool: &SqlitePool,
    url: &Url,
    db_event: DbEvent,
) -> Result<(), Error> {
    if db_event.pubkey == keys.public_key() {
        handle_user_contact_list(output, keys, pool, url, db_event).await?;
    } else {
        handle_other_contact_list(db_event).await?;
    }
    Ok(())
}

async fn handle_user_contact_list(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    pool: &SqlitePool,
    _url: &Url,
    db_event: DbEvent,
) -> Result<(), Error> {
    tracing::debug!("Received a ContactList");

    let filtered_contacts: Vec<_> = db_event
        .tags
        .iter()
        .filter_map(|t| DbContact::from_tag(t).ok())
        .filter(|c| c.pubkey() != &keys.public_key())
        .collect();

    for db_contact in &filtered_contacts {
        DbContact::upsert_contact(pool, db_contact).await?;
        let _ = output
            .send(BackendEvent::ContactCreated(db_contact.to_owned()))
            .await;
    }

    let _ = output.send(BackendEvent::ReceivedContactList).await;

    Ok(())
}

async fn handle_other_contact_list(_db_event: DbEvent) -> Result<(), Error> {
    // Others ContactList That Im in
    // which means that someone else added me to their contact list
    // so I could build a followers list from this
    tracing::info!("*** Others ContactList That Im in ***");
    Ok(())
}
