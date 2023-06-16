use chrono::Utc;
use nostrtalk::{
    db::{DbContact, DbEvent},
    net::handle_event,
};
use url::Url;

use super::*;
use crate::helpers::spawn_app;
use helpers::*;

/// Tests for Received event of Kind::ContactList

/// Database with no contacts
/// 1. Received contact list with no contacts. Just ok
#[tokio::test]
async fn contact_list_empty_bd_no_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let contacts = vec![];
    let ns_event = users_contact_list_event(&test_app.keys, contacts.clone().into_iter());
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
    assert_event_and_contacts(&test_app, &event_hash, 0).await;

    assert_received_contact_list_event(&mut rx).await;
}

/// 2. Received contact list with some contacts, insert all. Check if they were inserted
#[tokio::test]
async fn contact_list_some_bd_no_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("Vaderzz")),
    ];
    let ns_event = users_contact_list_event(&test_app.keys, contacts.clone().into_iter());
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
    assert_event_and_contacts(&test_app, &event_hash, 2).await;

    for _ in 0..2 {
        assert_message_received(&test_app, &mut rx).await;
    }

    assert_received_contact_list_event(&mut rx).await;
}

/// Database with some contacts(already have contact list)
/// 1. Received contact list with no contacts. Just ok. Check if contacts are the same
#[tokio::test]
async fn contact_list_some_bd_with_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("Vaderzz")),
    ];
    test_app.insert_contacts(contacts).await;

    let (mut output, mut receiver) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();
    let contacts = vec![];
    let ns_event = users_contact_list_event(&test_app.keys, contacts.clone().into_iter());
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
    assert_event_and_contacts(&test_app, &event_hash, 2).await;

    assert_received_contact_list_event(&mut receiver).await;
}
/// 2. Received OLD contact list. Ignore it. Check if contacts are the same.
#[tokio::test]
async fn contact_list_old_bd_with_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(10);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("A_friend1")),
        make_random_contact(None),
        make_random_contact(Some("AnotherOne")),
    ];
    let ns_event = users_contact_list_event(&test_app.keys, contacts.into_iter());
    let first_event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    // Insert first contact list
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id.clone(),
        ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_event_and_contacts(&test_app, &first_event_hash, 4).await;

    // Test backend event messages received
    for _ in 0..4 {
        assert_message_received(&test_app, &mut rx).await;
    }
    assert_received_contact_list_event(&mut rx).await;

    // Insert second contact list that is older
    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("OtherFriendz")),
    ];
    let old_ns_event = users_contact_list_builder(contacts.into_iter());
    let time = Utc::now() - chrono::Duration::minutes(10);
    let old_ns_event = event_with_time(&test_app.keys, old_ns_event, time.naive_utc());
    let second_event_hash = old_ns_event.id.clone();
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url,
        subscription_id,
        old_ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    let second_event = DbEvent::fetch_hash(test_app.pool(), &second_event_hash)
        .await
        .unwrap();
    assert!(
        second_event.is_none(),
        "Old contact list event should not be stored"
    );

    // must not receive another message
    assert_channel_timeout(&mut rx).await;
}
/// 3. Received SAME contact list. Ignore it. Check if contacts are the same.
#[tokio::test]
async fn contact_list_same_bd_with_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(10);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("A_friend1")),
        make_random_contact(None),
        make_random_contact(Some("AnotherOne")),
    ];
    let ns_event = users_contact_list_event(&test_app.keys, contacts.into_iter());
    let first_event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    // Insert first contact list
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id.clone(),
        ns_event.clone(),
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_event_and_contacts(&test_app, &first_event_hash, 4).await;

    // Test backend event messages received
    for _ in 0..4 {
        assert_message_received(&test_app, &mut rx).await;
    }
    assert_received_contact_list_event(&mut rx).await;

    // Insert same contact list
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
    assert_event_and_contacts(&test_app, &first_event_hash, 4).await;

    let events = DbEvent::fetch(test_app.pool()).await.unwrap();
    assert_eq!(events.len(), 1, "Only one event should be stored");

    // must not receive another message
    assert_channel_timeout(&mut rx).await;
}
/// 4. Received NEW contact list. Replace all contacts with this new list. Check if all contacts were inserted and the old ones removed.
#[tokio::test]
async fn contact_list_new_bd_with_list_stored() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(10);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let first_contacts = vec![
        make_contact(
            "9e45b5e573adfb70be9f81e6f19e3df334fa24b3a7273859104d399ccbf64e94",
            Some("Vaderzzz"),
        ),
        make_contact(
            "dafb7c5a8d3a061a8254eb9ffb132cceec0b5080357531006e127263121e3adc",
            Some("Friendzin"),
        ),
    ];
    let ns_event = users_contact_list_event(&test_app.keys, first_contacts.clone().into_iter());
    let first_event_hash = ns_event.id.clone();
    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    // Insert first contact list
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id.clone(),
        ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_event_and_contacts(&test_app, &first_event_hash, 2).await;

    // Test backend event messages received
    for _ in 0..2 {
        assert_message_received(&test_app, &mut rx).await;
    }
    assert_received_contact_list_event(&mut rx).await;

    // Insert second contact list that is newer
    let contacts = vec![
        make_random_contact(None),
        make_random_contact(Some("A_friend1")),
        make_random_contact(None),
        make_random_contact(Some("AnotherOne")),
    ];
    let new_ns_event = users_contact_list_builder(contacts.into_iter());
    let time = Utc::now() + chrono::Duration::minutes(10);
    let new_ns_event = event_with_time(&test_app.keys, new_ns_event, time.naive_utc());
    let second_event_hash = new_ns_event.id.clone();
    let result = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url,
        subscription_id,
        new_ns_event,
    )
    .await;

    // ASSERT
    assert!(result.is_ok(), "Error handling event: {:?}", result.err());
    assert_event_and_contacts(&test_app, &second_event_hash, 4).await;

    // Test backend event messages received
    for _ in 0..4 {
        assert_message_received(&test_app, &mut rx).await;
    }
    assert_received_contact_list_event(&mut rx).await;

    for first_cs in first_contacts {
        let db_contacts = DbContact::fetch(test_app.pool(), test_app.cache_pool())
            .await
            .unwrap();
        assert!(
            !db_contacts.iter().any(|db_c| db_c.pubkey() == &first_cs.pk),
            "Old contact should be removed"
        );
    }

    let events = DbEvent::fetch(test_app.pool()).await.unwrap();
    assert_eq!(events.len(), 1, "Only one event should be stored");
}
