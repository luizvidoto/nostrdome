use crate::Error;
use futures_util::SinkExt;
use nostr::{secp256k1::XOnlyPublicKey, Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, TagInfo},
    net::BackendEvent,
    types::ChatMessage,
};

pub async fn handle_dm(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    url: &Url,
    db_event: &DbEvent,
) -> Result<(), Error> {
    tracing::debug!("handle_dm");
    let info = TagInfo::from_db_event(db_event)?;
    let contact_pubkey = info.contact_pubkey(keys)?;

    // Parse the event and insert the message into the database
    let db_message = insert_message_from_event(db_event, pool, &contact_pubkey).await?;

    // If the message is from the user to themselves, log the error and return None
    if db_message.from_pubkey() == db_message.to_pubkey() {
        tracing::debug!("Message is from the user to himself");
        return Ok(());
    }

    // Determine the contact's public key and whether the message is from the user
    let contact_pubkey = determine_sender_receiver(&db_message, &keys.public_key());

    tracing::debug!("fetch_or_create_contact");
    let db_contact = match DbContact::fetch_one(pool, cache_pool, &contact_pubkey).await? {
        Some(db_contact) => db_contact,
        None => {
            tracing::debug!("Creating new contact with pubkey: {}", &contact_pubkey);
            let db_contact = DbContact::new(&contact_pubkey);
            DbContact::upsert_contact(pool, &db_contact).await?;
            db_contact
        }
    };

    let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;

    let _ = output
        .send(BackendEvent::ReceivedDM {
            chat_message,
            db_contact,
            relay_url: url.to_owned(),
        })
        .await;

    Ok(())
}

async fn insert_message_from_event(
    db_event: &DbEvent,
    pool: &SqlitePool,
    contact_pubkey: &XOnlyPublicKey,
) -> Result<DbMessage, Error> {
    tracing::debug!("insert_message_from_event");
    let db_message = DbMessage::confirmed_message(db_event, contact_pubkey)?;
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    Ok(db_message.with_id(msg_id))
}

fn determine_sender_receiver(
    db_message: &DbMessage,
    user_pubkey: &XOnlyPublicKey,
) -> XOnlyPublicKey {
    tracing::debug!("determine_sender_receiver");
    if db_message.im_author(user_pubkey) {
        tracing::debug!("Message is from the user");
        db_message.to_pubkey()
    } else {
        tracing::debug!("Message is from contact");
        db_message.from_pubkey()
    }
}
