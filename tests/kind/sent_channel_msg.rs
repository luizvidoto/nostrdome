use nostrtalk::db::DbChannelMessage;
use nostrtalk::net::{process_message, ToBackend};
use nostrtalk::utils::channel_id_from_tags;

use super::*;
use crate::spawn_app;

// When there is a PendingEvent, it can be confirmed in two ways,
// either by receiving an OK message from the relay or by receiving the event itself

/// Tests for send event of Kind::ChannelMessage

/// When message is sent, it should be stored in memory as a PendingEvent
#[tokio::test]
async fn sent_channel_msg_no_confirmation_in_db() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let channel_id = nostr::EventId::from_hex(
        "8233a5d8e27a9415d22c974d70935011664ada55ae3152bd10d697d3a3c74f67",
    )
    .unwrap();

    let content: String = "Hey friendz!".into();
    let message = ToBackend::SendChannelMessage(channel_id.clone(), content.clone());

    // PERFORM
    let result = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_eq!(test_app.backend.pending_events.len(), 1);

    if let Some((_, first)) = &test_app.backend.pending_events.iter().next() {
        assert_eq!(&first.ns_event().content, &content);

        let channel_id_from_event = channel_id_from_tags(&first.ns_event().tags).unwrap();
        assert_eq!(&channel_id_from_event, &channel_id);

        let msgs = DbChannelMessage::fetch(test_app.pool(), &channel_id)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 0, "No message should be in the database");
    }

    if let Some(event) = rx.next().await {
        if let BackendEvent::PendingChannelMsg(sent_channel_id, chat_message) = &event {
            assert_eq!(sent_channel_id, &channel_id);
            assert_eq!(chat_message.content(), &content);
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}
