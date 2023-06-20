use futures_util::StreamExt;
use nostr::Keys;
use nostrtalk::{
    db::{ChannelCache, DbChannelMessage, MessageStatus},
    net::{handle_event, BackendEvent},
    types::{ChatMessage, SubName, UserMessage},
};
use url::Url;

use super::dm_helpers::*;
use crate::common::{make_channel_msg_event, random_contact_channel_msg_event};
use crate::spawn_app;

/// Tests for Received event of Kind::ChannelMessage

#[tokio::test]
async fn channel_msg_user_to_channel() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let cache = test_app.insert_random_channel_cache().await;
    let channel_id = cache.channel_id;

    let msg_content: String = "hey group!".into();
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
        if let BackendEvent::ChannelCacheUpdated(_) = event {
            // Ok
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

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

    let cache = test_app.insert_random_channel_cache().await;
    let channel_id = cache.channel_id;

    let msg_content: String = "hey group".into();
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
        if let BackendEvent::ChannelCacheUpdated(_) = event {
            // Ok
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

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

#[tokio::test]
async fn search_channel_msg() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(20);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let cache = test_app.insert_random_channel_cache().await;
    let channel_id = cache.channel_id;

    let subscription_id =
        nostr::SubscriptionId::new(SubName::src_channel_details(&channel_id).to_string());
    let messages = vec!["hey group", "hey john", "yo", "lets go", "everyone"];

    // PERFORM
    for msg in &messages {
        let result = handle_event(
            &mut output,
            &test_app.keys,
            &mut test_app.backend,
            url.clone(),
            subscription_id.clone(),
            random_contact_channel_msg_event(&channel_id, msg),
        )
        .await;
        assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    }

    // ASSERT
    let db_msgs = DbChannelMessage::fetch(test_app.pool(), &channel_id)
        .await
        .unwrap();
    assert_eq!(db_msgs.len(), messages.len());

    let cache = ChannelCache::fetch_by_channel_id(test_app.cache_pool(), &channel_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cache.members.len(), messages.len());

    for (idx, msg) in messages.iter().enumerate() {
        if let Some(event) = rx.next().await {
            if let BackendEvent::ChannelCacheUpdated(cache) = &event {
                assert_eq!(cache.members.len(), idx + 1);
                assert_eq!(&cache.channel_id, &channel_id);
            } else {
                panic!("Unexpected event: {:?}", event);
            }
        }

        if let Some(event) = rx.next().await {
            if let BackendEvent::ReceivedChannelMessage(rcv_channel_id, chat_message) = &event {
                assert_eq!(rcv_channel_id, &channel_id);
                assert_eq!(&chat_message.content(), msg);
            } else {
                panic!("Unexpected event: {:?}", event);
            }
        }
    }
}
