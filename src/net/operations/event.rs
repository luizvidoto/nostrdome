use futures::channel::mpsc;
use nostr_sdk::{Keys, RelayMessage, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelayResponse},
    error::Error,
    net::{
        events::{
            backend::{handle_recommend_relay, BackEndInput},
            nostr::NostrInput,
            Event,
        },
        operations::{
            contact_list::handle_contact_list, direct_message::handle_dm,
            metadata::handle_metadata_event,
        },
    },
    types::ChatMessage,
};

pub async fn insert_confirmed_event(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("insert_event");

    match ns_event.kind {
        nostr_sdk::Kind::ContactList => {
            // contact_list is validating differently
            handle_contact_list(ns_event, keys, pool, back_sender, nostr_sender, relay_url).await
        }
        other => {
            if let Some(db_event) = confirmed_event(ns_event, relay_url, pool).await? {
                match other {
                    nostr_sdk::Kind::Metadata => {
                        handle_metadata_event(pool, cache_pool, relay_url, db_event).await
                    }
                    nostr_sdk::Kind::EncryptedDirectMessage => {
                        handle_dm(pool, cache_pool, keys, relay_url, db_event).await
                    }
                    nostr_sdk::Kind::RecommendRelay => handle_recommend_relay(db_event).await,
                    // nostr_sdk::Kind::ChannelCreation => {}
                    // nostr_sdk::Kind::ChannelMetadata => {
                    // nostr_sdk::Kind::ChannelMessage => {}
                    // nostr_sdk::Kind::ChannelHideMessage => {}
                    // nostr_sdk::Kind::ChannelMuteUser => {}
                    // Kind::EventDeletion => {},
                    // Kind::PublicChatReserved45 => {},
                    // Kind::PublicChatReserved46 => {},
                    // Kind::PublicChatReserved47 => {},
                    // Kind::PublicChatReserved48 => {},
                    // Kind::PublicChatReserved49 => {},
                    // Kind::ZapRequest => {},
                    // Kind::Zap => {},
                    // Kind::MuteList => {},
                    // Kind::PinList => {},
                    // Kind::RelayList => {},
                    // Kind::Authentication => {},
                    _other_kind => insert_other_kind(db_event).await,
                }
            } else {
                return Ok(Event::None);
            }
        }
    }
}

async fn confirmed_event(
    ns_event: nostr_sdk::Event,
    relay_url: &Url,
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<Option<DbEvent>, Error> {
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);
    relay_response_ok(pool, relay_url, &db_event).await?;
    if rows_changed == 0 {
        tracing::info!("Event already in database");
        return Ok(None);
    }
    Ok(Some(db_event))
}

async fn insert_other_kind(db_event: DbEvent) -> Result<Event, Error> {
    Ok(Event::OtherKindEventInserted(db_event))
}

async fn insert_pending(
    ns_event: nostr_sdk::Event,
    pool: &SqlitePool,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<DbEvent, Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    pending_event = pending_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Err(Error::DuplicateEvent);
    }
    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }
    Ok(pending_event)
}

pub async fn insert_pending_metadata(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _metadata: nostr_sdk::Metadata,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    match insert_pending(ns_event, pool, nostr_sender).await {
        Ok(pending_event) => Ok(Event::PendingMetadata(pending_event)),
        Err(Error::DuplicateEvent) => Ok(Event::None),
        Err(e) => Err(e),
    }
}

pub async fn insert_pending_contact_list(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _contact_list: Vec<DbContact>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    match insert_pending(ns_event, pool, nostr_sender).await {
        Ok(pending_event) => Ok(Event::PendingContactList(pending_event)),
        Err(Error::DuplicateEvent) => Ok(Event::None),
        Err(e) => Err(e),
    }
}

pub async fn insert_pending_dm(
    pool: &SqlitePool,
    keys: &Keys,
    ns_event: nostr_sdk::Event,
    db_contact: DbContact,
    content: String,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    match insert_pending(ns_event, pool, nostr_sender).await {
        Ok(pending_event) => {
            let pending_msg = DbMessage::new(&pending_event, db_contact.pubkey())?;
            let row_id = DbMessage::insert_message(pool, &pending_msg).await?;
            let pending_msg = pending_msg.with_id(row_id);

            let chat_message =
                ChatMessage::from_db_message_content(keys, &pending_msg, &db_contact, &content)?;
            // let db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
            Ok(Event::PendingDM((db_contact, chat_message)))
        }
        Err(Error::DuplicateEvent) => Ok(Event::None),
        Err(e) => Err(e),
    }
}

pub async fn on_relay_message(
    pool: &SqlitePool,
    relay_url: &Url,
    relay_message: &RelayMessage,
) -> Result<BackEndInput, Error> {
    tracing::debug!("New relay message: {}", relay_url);
    tracing::debug!("{:?}", relay_message);
    let event = match relay_message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message,
        } => {
            if *status == false {
                return Ok(BackEndInput::FailedToSendEvent {
                    relay_url: relay_url.to_owned(),
                    event_hash: event_hash.to_owned(),
                    status: status.to_owned(),
                    message: message.to_owned(),
                });
            }

            tracing::debug!("Relay message: Ok");
            let db_event = confirm_event(pool, event_hash, relay_url).await?;
            BackEndInput::RelayConfirmation(db_event)
        }
        RelayMessage::EndOfStoredEvents(subscription_id) => {
            tracing::debug!("Relay message: EOSE. ID: {}", subscription_id);
            BackEndInput::Ok(Event::EndOfStoredEvents((
                relay_url.to_owned(),
                subscription_id.to_owned(),
            )))
        }
        RelayMessage::Event {
            subscription_id,
            event,
        } => {
            tracing::debug!("Relay message: Event. ID: {}", subscription_id);
            tracing::debug!("{:?}", event);
            BackEndInput::Ok(Event::None)
        }
        RelayMessage::Notice { message } => {
            tracing::info!("Relay message: Notice: {}", message);
            BackEndInput::Ok(Event::None)
        }
        RelayMessage::Auth { challenge } => {
            tracing::warn!("Relay message: Auth Challenge: {}", challenge);
            BackEndInput::Ok(Event::None)
        }
        RelayMessage::Count {
            subscription_id: _,
            count,
        } => {
            tracing::info!("Relay message: Count: {}", count);
            BackEndInput::Ok(Event::None)
        }
        RelayMessage::Empty => {
            tracing::info!("Relay message: Empty");
            BackEndInput::Ok(Event::None)
        }
    };

    Ok(event)
}

async fn confirm_event(
    pool: &SqlitePool,
    event_hash: &nostr_sdk::EventId,
    relay_url: &Url,
) -> Result<DbEvent, Error> {
    let mut db_event = DbEvent::fetch_one(pool, event_hash)
        .await?
        .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
    if db_event.relay_url.is_none() {
        db_event = DbEvent::confirm_event(pool, relay_url, db_event).await?;
    }
    relay_response_ok(pool, relay_url, &db_event).await?;
    Ok(db_event)
}

pub async fn relay_response_ok(
    pool: &SqlitePool,
    relay_url: &nostr_sdk::Url,
    db_event: &DbEvent,
) -> Result<(), Error> {
    let relay_response = DbRelayResponse::ok(db_event.event_id()?, &db_event.event_hash, relay_url);
    DbRelayResponse::insert(pool, &relay_response).await?;
    Ok(())
}
