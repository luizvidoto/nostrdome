use crate::db::{DbContact, DbEvent, DbMessage, MessageTagInfo};
use crate::error::Error;
use crate::net::BackendEvent;
use crate::types::ChatMessage;

use futures_util::SinkExt;
use nostr::secp256k1::XOnlyPublicKey;
use nostr::{EventId, Keys};
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
    let Some((is_users, tag_info, chat_pubkey)) =
        verify_dm(&ns_event.id, &ns_event.pubkey, &ns_event.tags, keys)? else {
        return Ok(());
    };

    if let Some(db_event) = DbEvent::insert(pool, &url, &ns_event).await? {
        let (db_contact, chat_message) = insert_and_generate_msg(
            pool,
            cache_pool,
            keys,
            &db_event,
            chat_pubkey,
            is_users,
            tag_info,
        )
        .await?;

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

pub async fn pending_dm_confirmed(
    output: &mut futures::channel::mpsc::Sender<BackendEvent>,
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<(), Error> {
    let Some((is_users, tag_info, chat_pubkey)) =
        verify_dm(&db_event.event_hash, &db_event.pubkey, &db_event.tags, keys)? else {
        return Ok(());
    };

    let (_, chat_message) = insert_and_generate_msg(
        pool,
        cache_pool,
        keys,
        db_event,
        chat_pubkey,
        is_users,
        tag_info,
    )
    .await?;

    let _ = output.send(BackendEvent::ConfirmedDM(chat_message)).await;

    Ok(())
}

async fn insert_and_generate_msg(
    pool: &SqlitePool,
    cache_pool: &sqlx::Pool<sqlx::Sqlite>,
    keys: &Keys,
    db_event: &DbEvent,
    chat_pubkey: XOnlyPublicKey,
    is_users: bool,
    tag_info: MessageTagInfo,
) -> Result<(DbContact, ChatMessage), Error> {
    let db_message = DbMessage::insert_confirmed(pool, &db_event, &chat_pubkey, is_users).await?;
    let db_contact = DbContact::fetch_insert(pool, cache_pool, &db_message.chat_pubkey).await?;
    let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;
    let chat_message = ChatMessage::new(
        &db_message,
        &db_event.pubkey,
        &db_contact,
        &decrypted_content,
    );
    Ok((db_contact, chat_message))
}

fn verify_dm(
    event_hash: &EventId,
    event_pubkey: &XOnlyPublicKey,
    event_tags: &[nostr::Tag],
    keys: &Keys,
) -> Result<Option<(bool, MessageTagInfo, XOnlyPublicKey)>, Error> {
    let is_users = event_pubkey == &keys.public_key();
    let tag_info = MessageTagInfo::from_event_tags(&event_hash, &event_pubkey, &event_tags)?;
    let Some(chat_pubkey) = tag_info.chat_pubkey(keys) else {
        tracing::debug!("Message from anyone to unknown chat, ignoring");
        return Ok(None);
    };
    if &tag_info.to_pubkey == event_pubkey {
        tracing::debug!("Message is from the user to himself");
        return Ok(None);
    }

    Ok(Some((is_users, tag_info, chat_pubkey)))
}
