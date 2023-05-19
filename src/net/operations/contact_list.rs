use chrono::NaiveDateTime;
use futures::channel::mpsc;
use nostr_sdk::{Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent},
    error::Error,
    net::{
        events::{backend::BackEndInput, nostr::NostrInput, Event},
        operations::{contact::insert_contact_from_event, event::relay_response_ok},
    },
    utils::ns_event_to_millis,
};

pub async fn handle_contact_list(
    ns_event: nostr_sdk::Event,
    keys: &Keys,
    pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
) -> Result<Event, Error> {
    if ns_event.pubkey == keys.public_key() {
        handle_user_contact_list(ns_event, keys, pool, back_sender, nostr_sender, relay_url).await
    } else {
        handle_other_contact_list(ns_event)
    }
}

async fn handle_user_contact_list(
    ns_event: nostr_sdk::Event,
    keys: &Keys,
    pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::debug!("Received a ContactList");

    if let Some((remote_creation, db_event)) = last_kind_filtered(pool).await? {
        // if db_event.event_hash == ns_event.id {
        //     // if event already in the database, just confirmed it
        //     tracing::info!("ContactList already in the database");
        //     relay_response_ok(pool, relay_url, &db_event).await?;
        //     return Ok(Event::None);
        // }
        // if event is older than the last one, ignore it
        if remote_creation.timestamp_millis() > (ns_event_to_millis(ns_event.created_at)) {
            tracing::info!("ContactList is older than the last one");
            return Ok(Event::None);
        } else {
            // delete old and insert new contact list
            tracing::info!("ContactList is newer than the last one");
            DbEvent::delete(pool, db_event.event_id()?).await?;
        }
    } else {
        // if no contact list in the database, insert it
        tracing::info!("No ContactList in the database");
    }

    tracing::info!("Inserting contact list event");
    tracing::debug!("{:?}", ns_event);
    // create event struct
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    // insert into database
    let (row_id, _rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);
    relay_response_ok(pool, relay_url, &db_event).await?;

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
        match insert_contact_from_event(keys, pool, db_contact).await {
            Ok(event) => {
                if let Err(e) = back_sender.try_send(BackEndInput::Ok(event)) {
                    tracing::error!("Error sending message to backend: {:?}", e);
                }
            }
            Err(e) => {
                if let Err(e) = back_sender.try_send(BackEndInput::Error(e.to_string())) {
                    tracing::error!("Error sending message to backend: {:?}", e);
                }
            }
        }
    }

    if let Err(e) = nostr_sender.try_send(NostrInput::GetContactListProfiles(db_contacts.clone())) {
        tracing::error!("Error sending message to backend: {:?}", e);
    }

    Ok(Event::ReceivedContactList {
        contact_list: db_contacts,
        relay_url: relay_url.to_owned(),
    })
}

fn handle_other_contact_list(_ns_event: nostr_sdk::Event) -> Result<Event, Error> {
    // Others ContactList That Im in
    // which means that someone else added me to their contact list
    // so I could build a followers list from this
    tracing::info!("*** Others ContactList That Im in ***");
    Ok(Event::None)
}

async fn last_kind_filtered(pool: &SqlitePool) -> Result<Option<(NaiveDateTime, DbEvent)>, Error> {
    let last_event = match DbEvent::fetch_last_kind(pool, nostr_sdk::Kind::ContactList).await? {
        Some(last_event) => last_event,
        None => return Ok(None),
    };

    match (last_event.remote_creation(), last_event.event_id()) {
        (Some(remote_creation), Ok(_)) => Ok(Some((remote_creation, last_event))),
        _ => Ok(None),
    }
}
