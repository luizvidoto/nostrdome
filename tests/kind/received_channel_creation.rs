use futures_util::StreamExt;
use nostrtalk::{
    db::ChannelCache,
    net::{handle_event, BackendEvent},
    types::ChannelMetadata,
};
use url::Url;

use crate::common::make_channel_creation_event;
use crate::spawn_app;

#[tokio::test]
async fn channel_creation_ok() {
    // PREPARE
    let mut test_app = spawn_app().await;
    let (mut output, mut rx) = futures::channel::mpsc::channel(5);
    let url = Url::parse("ws://192.168.15.15:8080").unwrap();

    let channel_name: String = "Test Channel".into();
    let metadata = ChannelMetadata::new().name(&channel_name);
    let ns_event = make_channel_creation_event(&test_app.keys, &metadata);
    let channel_id = ns_event.id.clone();
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

    let cache = ChannelCache::fetch_by_channel_id(test_app.cache_pool(), &channel_id)
        .await
        .unwrap();

    let Some(cache) = cache else {
        panic!("Channel cache not found")
    };

    assert_eq!(&cache.creator_pubkey, &test_app.keys.public_key());
    assert_eq!(cache.metadata.name, Some(channel_name.clone()));

    if let Some(event) = rx.next().await {
        if let BackendEvent::ChannelCacheUpdated(rcv_cache) = &event {
            assert_eq!(&rcv_cache.channel_id, &channel_id);
            assert_eq!(rcv_cache.metadata.name, Some(channel_name));
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}
