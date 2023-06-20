use nostr::Keys;
use nostrtalk::db::{DbContact, MessageStatus};
use nostrtalk::net::handle_event;
use nostrtalk::types::{ChatMessage, UserMessage};
use url::Url;

use super::dm_helpers::*;
use super::*;
use crate::common::make_dm_event;
use crate::spawn_app;

/// Tests for Received event of Kind::EncryptedDirectMessage

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

    let db_event = assert_dm_in_database(&test_app, &event_hash, 1, &msg_content).await;

    if let Some(event) = rx.next().await {
        if let BackendEvent::ReceivedDM {
            relay_url,
            db_contact: _,
            chat_message,
        } = &event
        {
            assert_eq!(relay_url, &url);

            match chat_message {
                ChatMessage::ContactMessage { .. } => {
                    panic!("Wrong chat message type")
                }
                ChatMessage::UserMessage(user) => match user {
                    UserMessage::Pending { .. } => panic!("Wrong user message type"),
                    UserMessage::Confirmed {
                        content,
                        event_id,
                        status,
                        ..
                    } => {
                        assert_eq!(content, &msg_content);
                        assert_eq!(status, &MessageStatus::Delivered);
                        assert_eq!(&db_event.event_id, event_id);
                    }
                },
            }
        } else {
            panic!("Wrong event received: {:?}", event);
        }
    }
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

    let db_event = assert_dm_in_database(&test_app, &event_hash, 1, &msg_content).await;

    if let Some(event) = rx.next().await {
        if let BackendEvent::ReceivedDM {
            relay_url,
            db_contact,
            chat_message,
        } = &event
        {
            assert_eq!(relay_url, &url);
            match chat_message {
                ChatMessage::UserMessage(_) => panic!("Wrong chat message type"),
                ChatMessage::ContactMessage {
                    content,
                    author,
                    event_id,
                    status,
                    ..
                } => {
                    assert_eq!(db_contact.pubkey(), author);
                    assert_eq!(content, &msg_content);
                    assert_eq!(status, &MessageStatus::Delivered);
                    assert_eq!(&db_event.event_id, event_id);
                }
            }
        } else {
            panic!("Wrong event received: {:?}", event);
        }
    }
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

/// The message is from another user to user, contact not in db -> add contact and store message
#[tokio::test]
async fn dm_message_anyone_to_user_new_contact() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, _) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let sender_keys = Keys::generate();
    let msg_content: String = "yo jonas brow".into();
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

    _ = assert_dm_in_database(&test_app, &event_hash, 1, &msg_content).await;

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    if let Some(first) = contacts.first() {
        assert_eq!(first.pubkey(), &sender_keys.public_key());
    } else {
        panic!("No contact in the database");
    }
}
