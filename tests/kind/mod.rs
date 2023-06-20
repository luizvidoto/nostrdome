use std::time::Duration;

use futures::channel::mpsc::Receiver;
use futures_util::StreamExt;
use nostr::Keys;
use nostrtalk::{net::BackendEvent, types::ChannelMetadata};

mod contact_list_helpers;
mod dm_helpers;
mod received_channel_creation;
mod received_channel_metadata;
mod received_channel_msg;
mod received_contact_list;
mod received_dm;
mod sent_channel_msg;
mod sent_contact_list;
mod sent_dm;

/// The channel must not receive a message within the timeout duration
pub async fn assert_channel_timeout(rx: &mut Receiver<BackendEvent>) {
    let timeout_duration = Duration::from_secs(1);
    let result = tokio::time::timeout(timeout_duration, rx.next()).await;
    match result {
        Ok(Some(_)) => panic!("Unexpectedly received a message"),
        Ok(None) => panic!("Channel was closed unexpectedly"),
        Err(_) => (), // timeout occurred, this means that no message was received which is what we expected
    }
}
