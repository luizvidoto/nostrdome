use crate::db::{DbChat, DbContact, DbMessage, DbRelayResponse, MessageStatus};
use crate::error::Error;
use crate::net::database::database_connect;
use crate::net::nostr_client::nostr_client_connect;
use crate::types::ChatMessage;
use iced::Subscription;
use nostr_sdk::{Client, Contact, Keys};
use sqlx::SqlitePool;
use std::time::Duration;

mod back_channel;
mod contact;
mod relay;

pub(crate) mod database;
pub(crate) mod nostr_client;
pub(crate) use self::back_channel::{BackEndConnection, Connection};

pub fn backend_connect(
    keys: &Keys,
    db_conn: &BackEndConnection<database::Message>,
    ns_conn: &BackEndConnection<nostr_client::Message>,
) -> Vec<Subscription<Event>> {
    // EVENTO pode ser enum com dois tipos e o cliente escolhe qual quer ouvir
    // EventKind::Database
    // EventKind::NostrClient
    let database_sub = database_connect(keys, db_conn).map(|event| {
        if let database::Event::Error(e) = event {
            Event::Error(e)
        } else {
            Event::DbEvent(event)
        }
    });
    let nostr_client_sub = nostr_client_connect(keys, ns_conn).map(|event| {
        if let nostr_client::Event::Error(e) = event {
            Event::Error(e)
        } else {
            Event::NostrClientEvent(event)
        }
    });

    vec![database_sub, nostr_client_sub]
}

async fn _fetch_contacts_from_relays(nostr_client: &Client) -> Result<Vec<Contact>, Error> {
    let contacts = nostr_client
        .get_contact_list(Some(Duration::from_secs(10)))
        .await?;
    Ok(contacts)
}

async fn add_to_unseen_count(
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<DbContact, Error> {
    db_contact = DbContact::add_to_unseen_count(pool, &mut db_contact).await?;
    Ok(db_contact)
}

async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<(DbContact, Vec<ChatMessage>), Error> {
    tracing::info!("Fetching chat messages");
    let own_pubkey = keys.public_key();
    let chat = DbChat::new(&own_pubkey, db_contact.pubkey());
    let mut db_messages = chat.fetch_chat(pool).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for m in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        m.update_status(MessageStatus::Seen);
        DbMessage::update_message_status(pool, m).await?;
    }

    tracing::info!("Decrypting messages");
    for m in &mut db_messages {
        let content = m.decrypt_message(keys)?;
        let is_from_user = m.im_author(&keys.public_key());
        let chat_message = ChatMessage::from_db_message(&m, is_from_user, &db_contact, &content)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok((db_contact, chat_messages))
}

async fn fetch_relays_responses(
    pool: &SqlitePool,
    event_id: i64,
) -> Result<Vec<DbRelayResponse>, Error> {
    let responses = DbRelayResponse::fetch_by_event(pool, event_id).await?;

    Ok(responses)
}

async fn _send_contact_list(client: &Client, list: &[DbContact]) -> Result<Event, Error> {
    let c_list: Vec<_> = list.iter().map(|c| c.into()).collect();
    let _event_id = client.set_contact_list(c_list).await?;

    Ok(Event::None)
}

#[derive(Debug, Clone)]
pub enum Event {
    DbEvent(database::Event),
    NostrClientEvent(nostr_client::Event),
    Error(String),
    None,
}

const APP_TICK_INTERVAL_MILLIS: u64 = 50;
