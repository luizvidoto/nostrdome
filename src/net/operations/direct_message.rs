use crate::{db::TagInfo, Error};
use futures_util::SinkExt;
use nostr::{Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage},
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
    let is_users = db_event.pubkey == keys.public_key();
    let tag_info = TagInfo::from_db_event(db_event)?;
    let contact_pubkey = tag_info.contact_pubkey(keys)?;

    // If the message is from the user to themselves, log the error and return None
    if tag_info.to_pubkey == db_event.pubkey {
        tracing::debug!("Message is from the user to himself");
        return Ok(());
    }
    // Parse the event and insert the message into the database
    let db_message = DbMessage::insert_confirmed(pool, db_event, &contact_pubkey, is_users).await?;

    let db_contact = DbContact::fetch_insert(pool, cache_pool, &db_message.contact_pubkey).await?;

    let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
    let chat_message = ChatMessage::new(&db_message, &db_contact, &decrypted_content)?;

    let _ = output
        .send(BackendEvent::ReceivedDM {
            chat_message,
            db_contact,
            relay_url: url.to_owned(),
        })
        .await;

    Ok(())
}
