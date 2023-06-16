use crate::db::{DbContact, DbEvent, DbMessage, MessageTagInfo};
use crate::error::Error;
use crate::net::BackendEvent;
use crate::types::ChatMessage;

use futures_util::SinkExt;
use nostr::Keys;
use sqlx::SqlitePool;
use url::Url;

pub async fn handle_dm(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    url: &Url,
    ns_event: nostr::Event,
) -> Result<(), Error> {
    let is_users = ns_event.pubkey == keys.public_key();
    let tag_info = MessageTagInfo::from_event_tags(&ns_event.id, &ns_event.pubkey, &ns_event.tags)?;

    let Some(chat_pubkey) = tag_info.chat_pubkey(keys) else {
        tracing::debug!("Message from anyone to unknown chat, ignoring");
        return Ok(());
    };

    // If the message is from the user to themselves, log the error and return None
    if tag_info.to_pubkey == ns_event.pubkey {
        tracing::debug!("Message is from the user to himself");
        return Ok(());
    }

    if let Some(db_event) = DbEvent::insert(pool, &url, &ns_event).await? {
        let db_message =
            DbMessage::insert_confirmed(pool, &db_event, &chat_pubkey, is_users).await?;

        let db_contact = DbContact::fetch_insert(pool, cache_pool, &db_message.chat_pubkey).await?;

        let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
        let chat_message = ChatMessage::new(
            &db_message,
            &db_event.pubkey,
            &db_contact,
            &decrypted_content,
        );

        let _ = output
            .send(BackendEvent::ReceivedDM {
                chat_message,
                db_contact,
                relay_url: url.to_owned(),
            })
            .await;
    }

    Ok(())
}
