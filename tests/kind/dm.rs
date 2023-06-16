use futures::channel::mpsc::Receiver;
use futures_util::StreamExt;
use nostr::{secp256k1::XOnlyPublicKey, EventBuilder, Keys};
use nostrtalk::{
    db::{DbEvent, DbMessage, MessageStatus, MessageTagInfo},
    net::{handle_event, BackendEvent},
};
use url::Url;

use super::*;
use crate::helpers::{spawn_app, TestApp};

async fn assert_received_dm_event(
    rx: &mut Receiver<BackendEvent>,
    msg_author: &XOnlyPublicKey,
    url: &Url,
    chat_pubkey: &XOnlyPublicKey,
    msg_content: &str,
    is_users: bool,
) {
    if let Some(event) = rx.next().await {
        if let BackendEvent::ReceivedDM {
            relay_url,
            db_contact,
            chat_message,
        } = &event
        {
            assert_eq!(relay_url, url);
            assert_eq!(db_contact.pubkey(), chat_pubkey);
            assert_eq!(&chat_message.content, &msg_content);
            assert_eq!(&chat_message.author, msg_author);
            assert_eq!(chat_message.is_users, is_users);
            assert_eq!(chat_message.status, MessageStatus::Delivered);
        } else {
            panic!("Wrong event received: {:?}", event);
        }
    }
}

async fn assert_dm_in_database(
    test_app: &TestApp,
    event_hash: &nostr::EventId,
    contacts_len: usize,
    msg_content: &str,
) {
    let db_event = DbEvent::fetch_hash(test_app.pool(), &event_hash)
        .await
        .unwrap();
    assert!(db_event.is_some(), "Event should be in the database");

    let messages = DbMessage::fetch(test_app.pool()).await.unwrap();
    assert_eq!(
        messages.len(),
        contacts_len,
        "Wrong number of messages in the database"
    );
    if let Some(first) = messages.first() {
        let db_event = db_event.unwrap();
        assert_eq!(first.event_id, db_event.event_id);
        assert_eq!(first.encrypted_content, db_event.content);

        let tag_info =
            MessageTagInfo::from_event_tags(&db_event.event_hash, &db_event.pubkey, &db_event.tags)
                .unwrap();
        let decrypted_content = first.decrypt_message(&test_app.keys, &tag_info).unwrap();
        assert_eq!(decrypted_content, msg_content);
    }
}

async fn assert_dm_not_stored(test_app: &TestApp, event_hash: &nostr::EventId) {
    let db_event = DbEvent::fetch_hash(test_app.pool(), &event_hash)
        .await
        .unwrap();
    assert!(db_event.is_none(), "Event should not be in the database");

    let messages = DbMessage::fetch(test_app.pool()).await.unwrap();
    assert_eq!(
        messages.len(),
        0,
        "Wrong number of messages in the database"
    );
}

/// Tests for Received event of Kind::EncryptedDirectMessage

fn make_dm_event(
    sender_keys: &Keys,
    receiver_pubkey: XOnlyPublicKey,
    content: &str,
) -> nostr::Event {
    let builder =
        EventBuilder::new_encrypted_direct_msg(sender_keys, receiver_pubkey, content).unwrap();
    let event = builder.to_event(&sender_keys).unwrap();
    event
}

/// The message is from the user to himself -> ignore it
#[tokio::test]
async fn dm_message_user_to_himself() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let msg_content: String = "Hello, myself!".into();
    let ns_event = make_dm_event(&test_app.keys, test_app.keys.public_key(), &msg_content);
    let event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url,
        subscription_id,
        ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());

    assert_dm_not_stored(&test_app, &event_hash).await;

    assert_channel_timeout(&mut rx).await;
}

/// The message is from the user to another user -> store it
#[tokio::test]
async fn dm_message_user_to_another() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let receiver_keys = Keys::generate();
    let msg_content: String = "hey man".into();
    let ns_event = make_dm_event(&test_app.keys, receiver_keys.public_key(), &msg_content);
    let event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id,
        ns_event.clone(),
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());

    assert_dm_in_database(&test_app, &event_hash, 1, &msg_content).await;

    assert_received_dm_event(
        &mut rx,
        &test_app.keys.public_key(),
        &url,
        &receiver_keys.public_key(),
        &msg_content,
        true,
    )
    .await;
}

/// The message is from another user to the user -> store it
#[tokio::test]
async fn dm_message_anyone_to_user() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let sender_keys = Keys::generate();
    let msg_content: String = "yo jonas".into();
    let ns_event = make_dm_event(&sender_keys, test_app.keys.public_key(), &msg_content);
    let event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id,
        ns_event.clone(),
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());

    assert_dm_in_database(&test_app, &event_hash, 1, &msg_content).await;

    assert_received_dm_event(
        &mut rx,
        &sender_keys.public_key(),
        &url,
        &sender_keys.public_key(),
        &msg_content,
        false,
    )
    .await;
}

/// The message is from another user to another user -> ignore it, it shouldnt be possible
#[tokio::test]
async fn dm_message_anyone_to_another() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let sender_keys = Keys::generate();
    let receiver_keys = Keys::generate();
    let ns_event = make_dm_event(&sender_keys, receiver_keys.public_key(), "nice brow");
    let event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url,
        subscription_id,
        ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());

    assert_dm_not_stored(&test_app, &event_hash).await;

    assert_channel_timeout(&mut rx).await;
}
