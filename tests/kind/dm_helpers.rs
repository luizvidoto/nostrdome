use futures::channel::mpsc::Receiver;
use futures_util::StreamExt;
use nostr::secp256k1::XOnlyPublicKey;
use nostrtalk::{
    db::{DbEvent, DbMessage, MessageStatus, MessageTagInfo},
    net::BackendEvent,
};
use url::Url;

use crate::helpers::TestApp;

pub async fn assert_received_dm_event(
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

pub async fn assert_dm_in_database(
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

pub async fn assert_dm_not_stored(test_app: &TestApp, event_hash: &nostr::EventId) {
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
