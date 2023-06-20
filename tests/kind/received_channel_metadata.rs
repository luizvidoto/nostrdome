use futures_util::StreamExt;
use nostrtalk::{
    db::ChannelCache,
    net::{handle_event, BackendEvent},
};
use url::Url;

use super::*;
use crate::common::{make_channel_creation_event, make_channel_metadata_event};
use crate::spawn_app;

#[tokio::test]
async fn channel_metadata_ok() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let channel_name: String = "Test Channel".into();
    let metadata = ChannelMetadata::new().name(&channel_name);
    let creation_event = make_channel_creation_event(&test_app.keys, &metadata);
    let channel_id = creation_event.id.clone();

    let update_channel_name: String = "Updated Channel".into();
    let update_metadata = ChannelMetadata::new().name(&update_channel_name);
    let update_event =
        make_channel_metadata_event(&test_app.keys, &channel_id, Some(&url), &update_metadata);
    let update_event_hash = update_event.id.clone();

    let subscription_id = nostr::SubscriptionId::new("testing");

    // PERFORM
    let result_1 = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id.clone(),
        creation_event,
    )
    .await;

    let result_2 = handle_event(
        &mut output,
        &test_app.keys,
        &mut test_app.backend,
        url.clone(),
        subscription_id.clone(),
        update_event,
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

    let cache = ChannelCache::fetch_by_channel_id(test_app.cache_pool(), &channel_id)
        .await
        .unwrap();

    let Some(cache) = cache else {
        panic!("Channel cache not found")
    };

    assert_eq!(&cache.creator_pubkey, &test_app.keys.public_key());
    assert_eq!(cache.metadata.name, Some(update_channel_name.clone()));

    if let Some(event) = rx.next().await {
        if let BackendEvent::ChannelCacheUpdated(first_rcv_cache) = &event {
            assert_eq!(&first_rcv_cache.channel_id, &channel_id);
            assert_eq!(first_rcv_cache.updated_event_hash, None);
            assert_eq!(first_rcv_cache.metadata.name, Some(channel_name));
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }

    if let Some(event) = rx.next().await {
        if let BackendEvent::ChannelCacheUpdated(second_rcv_cache) = &event {
            assert_eq!(&second_rcv_cache.channel_id, &channel_id);
            assert_eq!(second_rcv_cache.updated_event_hash, Some(update_event_hash));
            assert_eq!(second_rcv_cache.metadata.name, Some(update_channel_name));
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}
