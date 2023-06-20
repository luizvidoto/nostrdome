use nostrtalk::{
    db::MessageStatus,
    net::handle_event,
    types::{ChatMessage, UserMessage},
};
use url::Url;

use super::dm_helpers::*;
use super::*;
use crate::helpers::spawn_app;

/// Tests for Received event of Kind::ChannelMessage

#[tokio::test]
async fn channel_msg_user_to_channel() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let msg_content: String = "hey group!".into();
    let channel_id = nostr::EventId::from_hex(
        "8233a5d8e27a9415d22c974d70935011664ada55ae3152bd10d697d3a3c74f67",
    )
    .unwrap();
    let ns_event = make_channel_msg_event(&test_app.keys, &channel_id, Some(&url), &msg_content);
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

    let db_event =
        assert_channel_msg_in_database(&test_app, &event_hash, &channel_id, 1, &msg_content).await;

    if let Some(event) = rx.next().await {
        if let BackendEvent::ReceivedChannelMessage(rcv_channel_id, chat_message) = &event {
            assert_eq!(rcv_channel_id, &channel_id);

            if let ChatMessage::UserMessage(user_msg) = &chat_message {
                match user_msg {
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
                    UserMessage::Pending { .. } => {
                        panic!("Wrong chat message type: {:?}", chat_message);
                    }
                }
            } else {
                panic!("Wrong chat message type: {:?}", chat_message);
            }
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}

#[tokio::test]
async fn channel_msg_anyone_to_channel() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let sender_keys = Keys::generate();
    let msg_content: String = "hey group".into();
    let channel_id = nostr::EventId::from_hex(
        "8233a5d8e27a9415d22c974d70935011664ada55ae3152bd10d697d3a3c74f67",
    )
    .unwrap();
    let ns_event = make_channel_msg_event(&sender_keys, &channel_id, Some(&url), &msg_content);
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

    let db_event =
        assert_channel_msg_in_database(&test_app, &event_hash, &channel_id, 1, &msg_content).await;

    if let Some(event) = rx.next().await {
        if let BackendEvent::ReceivedChannelMessage(rcv_channel_id, chat_message) = &event {
            assert_eq!(rcv_channel_id, &channel_id);

            if let ChatMessage::ContactMessage {
                content,
                event_id,
                status,
                author,
                ..
            } = &chat_message
            {
                assert_eq!(author, &sender_keys.public_key());
                assert_eq!(content, &msg_content);
                assert_eq!(status, &MessageStatus::Delivered);
                assert_eq!(&db_event.event_id, event_id);
            } else {
                panic!("Wrong chat message type: {:?}", chat_message);
            }
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}
