use nostr::Keys;
use nostrtalk::net::handle_event;
use url::Url;

use super::dm_helpers::*;
use super::*;
use crate::helpers::spawn_app;

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
