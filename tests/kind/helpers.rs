use futures::channel::mpsc::Receiver;
use futures_util::StreamExt;
use nostr::EventId;
use nostrtalk::{
    db::{DbContact, DbEvent},
    net::BackendEvent,
};

use crate::helpers::TestApp;

pub async fn assert_event_and_contacts(
    test_app: &TestApp,
    event_hash: &EventId,
    contacts_len: usize,
) {
    let db_event = DbEvent::fetch_hash(test_app.pool(), &event_hash)
        .await
        .unwrap();

    assert!(db_event.is_some(), "Event not found in database");

    let db_contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(
        db_contacts.len(),
        contacts_len,
        "Wrong number of contacts inserted"
    );
}

pub async fn assert_received_contact_list_event(receiver: &mut Receiver<BackendEvent>) {
    if let Some(BackendEvent::ReceivedContactList) = receiver.next().await {
        // Ok
        return;
    } else {
        panic!("Did not receive ReceivedContactList message");
    }
}

pub async fn assert_message_received(test_app: &TestApp, receiver: &mut Receiver<BackendEvent>) {
    if let Some(message) = receiver.next().await {
        assert_contact_with_message(test_app, message).await;
    } else {
        panic!("Did not receive expected message");
    }
}

pub async fn assert_contact_with_message(test_app: &TestApp, message: BackendEvent) {
    if let BackendEvent::ContactCreated(contact) = message {
        let db_contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
            .await
            .unwrap();

        assert!(
            db_contacts.iter().any(|ct| ct.pubkey() == contact.pubkey()),
            "Contact not found in database"
        );
    } else {
        panic!("Did not receive expected message");
    }
}
