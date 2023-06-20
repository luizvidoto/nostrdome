use nostrtalk::db::{DbContact, DbMessage};
use nostrtalk::net::{process_message, ToBackend};

use super::*;
use crate::helpers::spawn_app;

// When there is a PendingEvent, it can be confirmed in two ways,
// either by receiving an OK message from the relay or by receiving the event itself

/// Tests for send event of Kind::EncryptedDirectMessage

/// When dm is sent, it should be stored in memory as a PendingEvent
#[tokio::test]
async fn sent_dm_no_confirmation_in_db() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);
    let contact = DbContact::new(&contact.pk);

    let content: String = "Hey amigo!".into();
    let message = ToBackend::SendDM(contact.clone(), content.clone());

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

    let msgs = DbMessage::fetch(test_app.pool()).await.unwrap();
    assert_eq!(msgs.len(), 0, "No message should be in the database");

    if let Some(event) = rx.next().await {
        if let BackendEvent::PendingDM(db_contact, chat_message) = &event {
            assert_eq!(db_contact.pubkey(), contact.pubkey());
            assert_eq!(chat_message.content(), &content);
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}
