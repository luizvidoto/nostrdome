use std::{str::FromStr, time::Duration};

use futures::channel::mpsc::Receiver;
use futures_util::StreamExt;
use nostr::{secp256k1::XOnlyPublicKey, types::channel_id, Contact, EventBuilder, Keys};
use nostrtalk::{
    net::BackendEvent,
    types::ChannelMetadata,
    utils::{
        channel_creation_builder, channel_metadata_builder, channel_msg_builder, naive_to_event_tt,
    },
};
use url::Url;

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

pub fn event_with_time(
    keys: &Keys,
    builder: EventBuilder,
    time: chrono::NaiveDateTime,
) -> nostr::Event {
    let mut ns_event = builder.to_unsigned_event(keys.public_key());
    ns_event.created_at = naive_to_event_tt(time);
    let updated_id = nostr::EventId::new(
        &keys.public_key(),
        ns_event.created_at,
        &ns_event.kind,
        &ns_event.tags,
        &ns_event.content,
    );
    ns_event.id = updated_id;
    let ns_event = ns_event.sign(keys).unwrap();
    ns_event
}

pub fn users_contact_list_builder<I>(contacts: I) -> nostr::EventBuilder
where
    I: IntoIterator<Item = Contact>,
{
    EventBuilder::set_contact_list(contacts.into_iter().collect())
}

pub fn users_contact_list_event<I>(keys: &Keys, contacts: I) -> nostr::Event
where
    I: IntoIterator<Item = Contact>,
{
    let builder = users_contact_list_builder(contacts);
    let event = builder.to_event(&keys).unwrap();
    event
}

pub fn make_contact(pubkey: &str, alias: Option<&str>) -> Contact {
    let pubkey = XOnlyPublicKey::from_str(pubkey).unwrap();
    let alias = alias.map(|s| s.to_string());
    Contact::new(pubkey, None, alias)
}

pub fn make_random_contact(alias: Option<&str>) -> Contact {
    let keys = Keys::generate();
    let alias = alias.map(|s| s.to_string());
    Contact::new(keys.public_key(), None, alias)
}

fn make_dm_event(
    sender_keys: &Keys,
    receiver_pubkey: XOnlyPublicKey,
    content: &str,
) -> nostr::Event {
    let builder =
        EventBuilder::new_encrypted_direct_msg(sender_keys, receiver_pubkey, content).unwrap();
    let event = builder.to_event(&sender_keys).unwrap();
    event
}

fn make_channel_msg_event(
    sender_keys: &Keys,
    channel_id: &nostr::EventId,
    recommended_relay: Option<&Url>,
    content: &str,
) -> nostr::Event {
    let builder = channel_msg_builder(channel_id, recommended_relay, content);
    let event = builder.to_event(sender_keys).unwrap();
    event
}

fn make_channel_creation_event(sender_keys: &Keys, metadata: &ChannelMetadata) -> nostr::Event {
    let builder = channel_creation_builder(metadata);
    let event = builder.to_event(sender_keys).unwrap();
    event
}

fn make_channel_metadata_event(
    sender_keys: &Keys,
    channel_id: &nostr::EventId,
    recommended_relay: Option<&Url>,
    metadata: &ChannelMetadata,
) -> nostr::Event {
    let builder = channel_metadata_builder(channel_id, recommended_relay, metadata);
    let event = builder.to_event(sender_keys).unwrap();
    event
}
