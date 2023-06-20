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

    if let Some(db_event) = DbEvent::insert(pool, url, &ns_event).await? {
        let db_message =
            DbMessage::insert_confirmed(pool, &db_event, &chat_pubkey, is_users).await?;
        let db_contact = DbContact::fetch_insert(pool, cache_pool, &db_message.chat_pubkey).await?;
        let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;

        let chat_message = if is_users {
            ChatMessage::confirmed_users(&db_message, &decrypted_content)
        } else {
            ChatMessage::confirmed_contacts(&db_message, &db_contact, &decrypted_content)
        };

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
    keys: &Keys,
    db_event: &DbEvent,
) -> Result<(), Error> {
    let Some((is_users, tag_info, chat_pubkey)) =
        verify_dm(&db_event.event_hash, &db_event.pubkey, &db_event.tags, keys)? else {
        return Ok(());
    };

    let db_message = DbMessage::insert_confirmed(pool, db_event, &chat_pubkey, is_users).await?;
    let decrypted_content = db_message.decrypt_message(keys, &tag_info)?;

    let _ = output
        .send(BackendEvent::ConfirmedDM(
            db_event.event_hash.to_owned(),
            db_message,
            decrypted_content,
        ))
        .await;

    Ok(())
}

fn verify_dm(
    event_hash: &EventId,
    event_pubkey: &XOnlyPublicKey,
    event_tags: &[nostr::Tag],
    keys: &Keys,
) -> Result<Option<(bool, MessageTagInfo, XOnlyPublicKey)>, Error> {
    let is_users = event_pubkey == &keys.public_key();

    let tag_info = MessageTagInfo::from_event_tags(event_hash, event_pubkey, event_tags)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::{EventId, Keys, Kind, Tag, Timestamp};

    #[test]
    fn test_verify_dm_to_self_message() {
        let sender_keys = Keys::generate();

        let event_pubkey = sender_keys.public_key();
        let event_tags = &[Tag::PubKey(sender_keys.public_key(), None)];
        let event_id = EventId::new(
            &event_pubkey,
            Timestamp::now(),
            &Kind::EncryptedDirectMessage, // replace this with an actual Kind
            event_tags,
            "some content",
        );

        let result = verify_dm(&event_id, &event_pubkey, event_tags, &sender_keys);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_verify_dm_unknown_chat() {
        let sender_keys = Keys::generate();
        let receiver_keys = Keys::generate();
        let user_keys = Keys::generate();

        let event_pubkey = sender_keys.public_key();
        let event_tags = &[Tag::PubKey(receiver_keys.public_key(), None)];
        let event_id = EventId::new(
            &event_pubkey,
            Timestamp::now(),
            &Kind::EncryptedDirectMessage, // replace this with an actual Kind
            event_tags,
            "some content",
        );

        let result = verify_dm(&event_id, &event_pubkey, event_tags, &user_keys);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_verify_dm_valid_received_message_from_anyone() {
        let sender_keys = Keys::generate();
        let user_keys = Keys::generate();

        let msg_tag_info = MessageTagInfo {
            from_pubkey: sender_keys.public_key(),
            to_pubkey: user_keys.public_key(),
        };

        let event_pubkey = sender_keys.public_key();
        let event_tags = &[Tag::PubKey(user_keys.public_key(), None)];
        let event_id = EventId::new(
            &event_pubkey,
            Timestamp::now(),
            &Kind::EncryptedDirectMessage, // replace this with an actual Kind
            event_tags,
            "some content",
        );

        let result = verify_dm(&event_id, &event_pubkey, event_tags, &user_keys);
        let result = result.unwrap();

        let Some((is_users, tag_info, chat_pubkey)) = result else {
            panic!("Expected Some");
        };

        assert_eq!(is_users, false);
        assert_eq!(tag_info.from_pubkey, msg_tag_info.from_pubkey);
        assert_eq!(tag_info.to_pubkey, msg_tag_info.to_pubkey);
        assert_eq!(
            tag_info.chat_pubkey(&user_keys),
            msg_tag_info.chat_pubkey(&user_keys)
        );
        assert_eq!(chat_pubkey, sender_keys.public_key());
    }

    #[test]
    fn test_verify_dm_valid_received_message_from_user_to_anyone() {
        let receiver_keys = Keys::generate();
        let user_keys = Keys::generate();

        let msg_tag_info = MessageTagInfo {
            from_pubkey: user_keys.public_key(),
            to_pubkey: receiver_keys.public_key(),
        };

        let event_pubkey = user_keys.public_key();
        let event_tags = &[Tag::PubKey(receiver_keys.public_key(), None)];
        let event_id = EventId::new(
            &event_pubkey,
            Timestamp::now(),
            &Kind::EncryptedDirectMessage, // replace this with an actual Kind
            event_tags,
            "some content",
        );

        let result = verify_dm(&event_id, &event_pubkey, event_tags, &user_keys);
        let result = result.unwrap();

        let Some((is_users, tag_info, chat_pubkey)) = result else {
            panic!("Expected Some");
        };

        assert_eq!(is_users, true);
        assert_eq!(tag_info.from_pubkey, msg_tag_info.from_pubkey);
        assert_eq!(tag_info.to_pubkey, msg_tag_info.to_pubkey);
        assert_eq!(
            tag_info.chat_pubkey(&user_keys),
            msg_tag_info.chat_pubkey(&user_keys)
        );
        assert_eq!(chat_pubkey, receiver_keys.public_key());
    }
}
