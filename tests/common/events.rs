use std::str::FromStr;

use nostr::{secp256k1::XOnlyPublicKey, Contact, EventBuilder, Keys};
use nostrtalk::{
    types::ChannelMetadata,
    utils::{
        channel_creation_builder, channel_metadata_builder, channel_msg_builder, naive_to_event_tt,
    },
};
use url::Url;

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

pub fn make_dm_event(
    sender_keys: &Keys,
    receiver_pubkey: XOnlyPublicKey,
    content: &str,
) -> nostr::Event {
    let builder =
        EventBuilder::new_encrypted_direct_msg(sender_keys, receiver_pubkey, content).unwrap();
    let event = builder.to_event(&sender_keys).unwrap();
    event
}

pub fn make_channel_msg_event(
    sender_keys: &Keys,
    channel_id: &nostr::EventId,
    recommended_relay: Option<&Url>,
    content: &str,
) -> nostr::Event {
    let builder = channel_msg_builder(channel_id, recommended_relay, content);
    let event = builder.to_event(sender_keys).unwrap();
    event
}

pub fn make_channel_creation_event(sender_keys: &Keys, metadata: &ChannelMetadata) -> nostr::Event {
    let builder = channel_creation_builder(metadata);
    let event = builder.to_event(sender_keys).unwrap();
    event
}

pub fn make_channel_metadata_event(
    sender_keys: &Keys,
    channel_id: &nostr::EventId,
    recommended_relay: Option<&Url>,
    metadata: &ChannelMetadata,
) -> nostr::Event {
    let builder = channel_metadata_builder(channel_id, recommended_relay, metadata);
    let event = builder.to_event(sender_keys).unwrap();
    event
}

pub fn random_contact_channel_msg_event(
    channel_id: &nostr::EventId,
    content: &str,
) -> nostr::Event {
    let keys = Keys::generate();
    make_channel_msg_event(&keys, channel_id, None, content)
}
