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
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("insert_event");
    match ns_event.kind {
        nostr_sdk::Kind::ContactList => {
            handle_contact_list(ns_event, keys, pool, back_sender, nostr_sender, relay_url).await
        }
        nostr_sdk::Kind::Metadata => handle_metadata_event(pool, keys, relay_url, ns_event).await,
        nostr_sdk::Kind::EncryptedDirectMessage => handle_dm(pool, keys, relay_url, ns_event).await,
        nostr_sdk::Kind::RecommendRelay => handle_recommend_relay(ns_event).await,
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
        _other_kind => insert_other_kind(ns_event, relay_url, pool).await,
    }
}

async fn insert_other_kind(
    ns_event: nostr_sdk::Event,
    relay_url: &nostr_sdk::Url,
    pool: &SqlitePool,
) -> Result<Event, Error> {
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);
    let _ev = relay_response_ok(pool, &db_event, relay_url).await?;
    if rows_changed == 0 {
        tracing::info!("Received duplicate event: {:?}", db_event.event_hash);
        return Ok(Event::None);
    }
    Ok(Event::OtherKindEventInserted(db_event))
}

async fn insert_pending(
    ns_event: &nostr_sdk::Event,
    pool: &SqlitePool,
) -> Result<(DbEvent, u8), Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    pending_event = pending_event.with_id(row_id);
    Ok((pending_event, rows_changed))
}

pub async fn insert_pending_metadata(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _metadata: nostr_sdk::Metadata,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let (pending_event, rows_changed) = insert_pending(&ns_event, pool).await?;
    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }
    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }
    Ok(Event::PendingMetadata(pending_event))
}

pub async fn insert_pending_contact_list(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _contact_list: Vec<DbContact>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let (pending_event, rows_changed) = insert_pending(&ns_event, pool).await?;
    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }
    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }
    Ok(Event::PendingContactList(pending_event))
}

pub async fn insert_pending_dm(
    pool: &SqlitePool,
    keys: &Keys,
    ns_event: nostr_sdk::Event,
    db_contact: DbContact,
    content: String,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let (pending_event, rows_changed) = insert_pending(&ns_event, pool).await?;
    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }

    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }

    let pending_msg = DbMessage::new(&pending_event, db_contact.pubkey())?;
    let row_id = DbMessage::insert_message(pool, &pending_msg).await?;
    let pending_msg = pending_msg.with_id(row_id);

    let chat_message =
        ChatMessage::from_db_message_content(keys, &pending_msg, &db_contact, &content)?;
    // let db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
    Ok(Event::PendingDM((db_contact, chat_message)))
}

pub async fn relay_response_ok(
    pool: &SqlitePool,
    db_event: &DbEvent,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::info!("Updating relay response");
    let mut relay_response = DbRelayResponse::from_response(
        true,
        db_event.event_id()?,
        &db_event.event_hash,
        relay_url,
        "",
    );
    let id = DbRelayResponse::insert(pool, &relay_response).await?;
    relay_response = relay_response.with_id(id);
    // update db_message ?
    Ok(Event::RelayConfirmation {
        relay_response,
        db_event: db_event.clone(),
        db_message: None,
        db_contact: None,
    })
}

pub async fn on_relay_message(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    relay_message: &RelayMessage,
) -> Result<Event, Error> {
    tracing::info!("New relay message: {}", relay_url);
    tracing::debug!("{:?}", relay_message);
    let event = match relay_message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message,
        } => {
            if *status == false {
                tracing::info!("Relay message not ok: {}", message);
                return Ok(Event::None);
            }

            tracing::info!("Relay message: Ok");
            let (db_event, db_message, db_contact) =
                confirm_event_and_message(pool, keys, event_hash, relay_url).await?;

            let mut relay_response = DbRelayResponse::from_response(
                *status,
                db_event.event_id()?,
                event_hash,
                relay_url,
                message,
            );
            let id = DbRelayResponse::insert(pool, &relay_response).await?;
            relay_response = relay_response.with_id(id);
            Event::RelayConfirmation {
                relay_response,
                db_event,
                db_message,
                db_contact,
            }
        }
        RelayMessage::EndOfStoredEvents(subscription_id) => {
            tracing::info!("Relay message: EOSE. ID: {}", subscription_id);
            Event::EndOfStoredEvents((relay_url.to_owned(), subscription_id.to_owned()))
        }
        RelayMessage::Event {
            subscription_id,
            event,
        } => {
            tracing::debug!("Relay message: Event. ID: {}", subscription_id);
            tracing::debug!("{:?}", event);
            Event::None
        }
        RelayMessage::Notice { message } => {
            tracing::info!("Relay message: Notice: {}", message);
            Event::None
        }
        RelayMessage::Auth { challenge } => {
            tracing::warn!("Relay message: Auth Challenge: {}", challenge);
            Event::None
        }
        RelayMessage::Count {
            subscription_id: _,
            count,
        } => {
            tracing::info!("Relay message: Count: {}", count);
            Event::None
        }
        RelayMessage::Empty => {
            tracing::info!("Relay message: Empty");
            Event::None
        }
    };

    Ok(event)
}

async fn confirm_event_and_message(
    pool: &SqlitePool,
    keys: &Keys,
    event_hash: &nostr_sdk::EventId,
    relay_url: &Url,
) -> Result<(DbEvent, Option<DbMessage>, Option<DbContact>), Error> {
    let mut db_event = DbEvent::fetch_one(pool, event_hash)
        .await?
        .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
    let mut pair = (None, None);
    if db_event.relay_url.is_none() {
        db_event = DbEvent::confirm_event(pool, relay_url, db_event).await?;

        if let nostr_sdk::Kind::EncryptedDirectMessage = db_event.kind {
            pair =
                if let Some(db_message) = DbMessage::fetch_one(pool, db_event.event_id()?).await? {
                    let confirmed_db_message =
                        DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
                    // add relay confirmation

                    if let Some(db_contact) =
                        DbContact::fetch_one(pool, &confirmed_db_message.contact_chat()).await?
                    {
                        let chat_message =
                            ChatMessage::from_db_message(keys, &confirmed_db_message, &db_contact)?;
                        let db_contact =
                            DbContact::new_message(pool, db_contact, &chat_message).await?;
                        (Some(confirmed_db_message), Some(db_contact))
                    } else {
                        (Some(confirmed_db_message), None)
                    }
                } else {
                    (None, None)
                };
        }
    }
    Ok((db_event, pair.0, pair.1))
}
