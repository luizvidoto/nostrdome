use nostrtalk::db::DbContact;
use nostrtalk::net::{process_message, ToBackend};

use super::*;
use crate::common::{make_contact, make_random_contact};
use crate::spawn_app;

// When there is a PendingEvent, it can be confirmed in two ways,
// either by receiving an OK message from the relay or by receiving the event itself

/// Tests for send event of Kind::ContactList

// Every contact operation should be sent to the network
// AddContact(DbContact),
// UpdateContact(DbContact),
// DeleteContact(DbContact),
// ImportContacts(Vec<DbContact>, bool),

#[tokio::test]
async fn add_new_contact() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);
    let contact = DbContact::new(&contact.pk);
    let message = ToBackend::AddContact(contact.clone());

    // PERFORM
    let result = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_eq!(test_app.backend.pending_events.len(), 1);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(contacts.len(), 1, "Wrong number of contacts inserted");

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactCreated(new_contact) = &event {
            assert_eq!(new_contact.pubkey(), contact.pubkey());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}

#[tokio::test]
async fn add_contact_same_as_user() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_contact(&test_app.keys.public_key().to_string(), None);
    let contact = DbContact::new(&contact.pk);
    let message = ToBackend::AddContact(contact.clone());

    // PERFORM
    let result = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    // ASSERT
    assert!(result.is_err(), "Should not add contact same as user");
    assert_eq!(test_app.backend.pending_events.len(), 0);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(
        contacts.len(),
        0,
        "Should not insert contact same pubkey as user"
    );

    assert_channel_timeout(&mut rx).await;
}

#[tokio::test]
async fn contact_must_be_unique() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);
    let contact = DbContact::new(&contact.pk);
    let message = ToBackend::AddContact(contact.clone());

    // PERFORM
    let result_1 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    let result_2 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    // ASSERT
    assert!(
        result_1.is_ok(),
        "Error handling event: {:?}",
        result_1.err()
    );
    assert!(result_2.is_err());

    assert_eq!(test_app.backend.pending_events.len(), 1);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(contacts.len(), 1, "Wrong number of contacts inserted");

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactCreated(new_contact) = &event {
            assert_eq!(new_contact.pubkey(), contact.pubkey());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    assert_channel_timeout(&mut rx).await;
}

#[tokio::test]
async fn add_and_update_contact() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);
    let db_contact = DbContact::new(&contact.pk);

    let contact_update = DbContact::new(&contact.pk);
    let contact_update =
        DbContact::edit_contact(contact_update, "Myfriendz", "ws://192.168.15.15").unwrap();

    let message_add = ToBackend::AddContact(db_contact.clone());
    let message_update = ToBackend::UpdateContact(contact_update.clone());

    // PERFORM
    let result_1 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message_add.clone(),
    )
    .await;

    let result_2 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message_update.clone(),
    )
    .await;

    // ASSERT
    assert!(
        result_1.is_ok(),
        "Error handling event: {:?}",
        result_1.err()
    );
    assert!(
        result_2.is_ok(),
        "Error handling event: {:?}",
        result_2.err()
    );

    assert_eq!(test_app.backend.pending_events.len(), 2);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(contacts.len(), 1, "Wrong number of contacts inserted");

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactCreated(new_contact) = &event {
            assert_eq!(new_contact.pubkey(), db_contact.pubkey());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactUpdated(new_contact) = &event {
            assert_eq!(new_contact.pubkey(), contact_update.pubkey());
            assert_eq!(new_contact.get_petname(), contact_update.get_petname());
            assert_eq!(new_contact.get_relay_url(), contact_update.get_relay_url());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    assert_channel_timeout(&mut rx).await;
}

#[tokio::test]
async fn update_contact() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);

    test_app.insert_contacts(vec![contact.clone()]).await;

    let contact_update = DbContact::new(&contact.pk);
    let contact_update =
        DbContact::edit_contact(contact_update, "Myfriendz", "ws://192.168.15.15").unwrap();

    let message = ToBackend::UpdateContact(contact_update.clone());

    // PERFORM
    let result_1 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    // ASSERT
    assert!(
        result_1.is_ok(),
        "Error handling event: {:?}",
        result_1.err()
    );

    assert_eq!(test_app.backend.pending_events.len(), 1);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(contacts.len(), 1, "Wrong number of contacts inserted");

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactUpdated(new_contact) = &event {
            assert_eq!(new_contact.pubkey(), contact_update.pubkey());
            assert_eq!(new_contact.get_petname(), contact_update.get_petname());
            assert_eq!(new_contact.get_relay_url(), contact_update.get_relay_url());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    assert_channel_timeout(&mut rx).await;
}

#[tokio::test]
async fn delete_contact() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let (tasks_tx, _) = tokio::sync::mpsc::channel(5);

    let contact = make_random_contact(None);
    let db_contact = DbContact::new(&contact.pk);
    test_app.insert_contacts(vec![contact.clone()]).await;

    let message = ToBackend::DeleteContact(db_contact.clone());

    // PERFORM
    let result_1 = process_message(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        &tasks_tx,
        message.clone(),
    )
    .await;

    // ASSERT
    assert!(
        result_1.is_ok(),
        "Error handling event: {:?}",
        result_1.err()
    );

    assert_eq!(test_app.backend.pending_events.len(), 1);

    let contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
        .await
        .unwrap();

    assert_eq!(contacts.len(), 0, "Contact not deleted");

    if let Some(event) = rx.next().await {
        if let BackendEvent::ContactDeleted(contact) = &event {
            assert_eq!(contact.pubkey(), db_contact.pubkey());
            assert_eq!(contact.get_petname(), db_contact.get_petname());
            assert_eq!(contact.get_relay_url(), db_contact.get_relay_url());
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    assert_channel_timeout(&mut rx).await;
}
