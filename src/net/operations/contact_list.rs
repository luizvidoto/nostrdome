use crate::{net::operations::event::confirmed_event, Error};
use chrono::NaiveDateTime;
use futures_util::SinkExt;
use nostr::{Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbRelayResponse},
    net::BackendEvent,
    utils::ns_event_to_millis,
};

pub async fn handle_contact_list(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    pool: &SqlitePool,
    url: &Url,
    ns_event: nostr::Event,
) -> Result<(), Error> {
    if ns_event.pubkey == keys.public_key() {
        handle_user_contact_list(output, keys, pool, url, ns_event).await?;
    } else {
        handle_other_contact_list(ns_event).await?;
    }
    Ok(())
}

async fn handle_user_contact_list(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    keys: &Keys,
    pool: &SqlitePool,
    url: &Url,
    ns_event: nostr::Event,
) -> Result<(), Error> {
    tracing::debug!("Received a ContactList");

    if let Some((remote_creation, db_event)) = last_kind_filtered(pool).await? {
        if db_event.event_hash == ns_event.id {
            // if event already in the database, just confirmed it
            tracing::info!("ContactList already in the database");
            DbRelayResponse::insert_ok(pool, url, &db_event).await?;
            return Ok(());
        }
        // if event is older than the last one, ignore it
        if remote_creation.timestamp_millis() > (ns_event_to_millis(ns_event.created_at)) {
            tracing::info!("ContactList is older than the last one");
            return Ok(());
        } else {
            // delete old and insert new contact list
            tracing::info!("ContactList is newer than the last one");
            DbEvent::delete(pool, db_event.event_id()?).await?;
        }
    } else {
        // if no contact list in the database, insert it
        tracing::info!("No ContactList in the database");
    }

    let db_event = confirmed_event(pool, url, ns_event)
        .await?
        .ok_or(Error::FailedToInsert)?;

    // contact list from event tags
    let db_contacts: Vec<_> = db_event
        .tags
        .iter()
        .filter_map(|t| DbContact::from_tag(t).ok())
        .collect();

    // Filter out contacts with the same public key as the user's public key
    let filtered_contacts: Vec<&DbContact> = db_contacts
        .iter()
        .filter(|c| c.pubkey() != &keys.public_key())
        .collect();

    if filtered_contacts.len() < db_contacts.len() {
        tracing::warn!("Error inserting contact: {:?}", Error::SameContactInsert);
    }

    for db_contact in &db_contacts {
        DbContact::upsert_contact(pool, db_contact).await?;
        let _ = output
            .send(BackendEvent::ContactCreated(db_contact.to_owned()))
            .await;
    }

    let _ = output
        .send(BackendEvent::ReceivedContactList {
            contact_list: db_contacts,
            relay_url: url.to_owned(),
        })
        .await;

    Ok(())
}

async fn handle_other_contact_list(_ns_event: nostr::Event) -> Result<(), Error> {
    // Others ContactList That Im in
    // which means that someone else added me to their contact list
    // so I could build a followers list from this
    tracing::info!("*** Others ContactList That Im in ***");
    Ok(())
}

async fn last_kind_filtered(pool: &SqlitePool) -> Result<Option<(NaiveDateTime, DbEvent)>, Error> {
    let last_event = match DbEvent::fetch_last_kind(pool, nostr::Kind::ContactList).await? {
        Some(last_event) => last_event,
        None => return Ok(None),
    };

    match (last_event.remote_creation(), last_event.event_id()) {
        (Some(remote_creation), Ok(_)) => Ok(Some((remote_creation, last_event))),
        _ => Ok(None),
    }
}
