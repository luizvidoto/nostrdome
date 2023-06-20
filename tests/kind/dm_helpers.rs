use nostrtalk::{
    db::{DbChannelMessage, DbEvent, DbMessage, MessageTagInfo},
    utils::channel_id_from_tags,
};

use crate::TestApp;

pub async fn assert_dm_in_database(
    test_app: &TestApp,
    event_hash: &nostr::EventId,
    messages_len: usize,
    msg_content: &str,
) -> DbEvent {
    let db_event = DbEvent::fetch_hash(test_app.pool(), &event_hash)
        .await
        .unwrap();
    assert!(db_event.is_some(), "Event should be in the database");

    let messages = DbMessage::fetch(test_app.pool()).await.unwrap();
    assert_eq!(
        messages.len(),
        messages_len,
        "Wrong number of messages in the database"
    );
    if let Some(first) = messages.first() {
        let db_event = db_event.clone().unwrap();
        assert_eq!(first.event_id, db_event.event_id);
        assert_eq!(first.encrypted_content, db_event.content);

        let tag_info =
            MessageTagInfo::from_event_tags(&db_event.event_hash, &db_event.pubkey, &db_event.tags)
                .unwrap();
        let decrypted_content = first.decrypt_message(&test_app.keys, &tag_info).unwrap();
        assert_eq!(decrypted_content, msg_content);
    }

    db_event.unwrap()
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

pub async fn assert_channel_msg_in_database(
    test_app: &TestApp,
    event_hash: &nostr::EventId,
    channel_id: &nostr::EventId,
    messages_len: usize,
    msg_content: &str,
) -> DbEvent {
    let db_event = DbEvent::fetch_hash(test_app.pool(), &event_hash)
        .await
        .unwrap();
    assert!(db_event.is_some(), "Event should be in the database");

    let messages = DbChannelMessage::fetch(test_app.pool(), channel_id)
        .await
        .unwrap();
    assert_eq!(
        messages.len(),
        messages_len,
        "Wrong number of messages in the database"
    );

    if let Some(first) = messages.first() {
        let db_event = db_event.clone().unwrap();
        assert_eq!(first.event_id, db_event.event_id);
        assert_eq!(first.content, msg_content);

        let tag_channel_id = channel_id_from_tags(&db_event.tags).unwrap();
        assert_eq!(&tag_channel_id, channel_id);
    }

    db_event.unwrap()
}
