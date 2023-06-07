use nostr::{secp256k1::XOnlyPublicKey, Filter, Kind, Timestamp};

use crate::db::DbContact;

pub fn contact_list_metadata(contact_list: &[DbContact]) -> Option<Filter> {
    if contact_list.is_empty() {
        return None;
    }

    let all_pubkeys = contact_list
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();

    Some(Filter::new().authors(all_pubkeys).kind(Kind::Metadata))
}
pub fn user_metadata(pubkey: XOnlyPublicKey, last_event_tt: u64) -> Filter {
    Filter::new()
        .author(pubkey.to_string())
        .kind(Kind::Metadata)
        .since(Timestamp::from(last_event_tt))
}

pub fn contact_list_filter(public_key: XOnlyPublicKey, last_event_tt: u64) -> Filter {
    let user_contact_list = Filter::new()
        .author(public_key.to_string())
        .kind(Kind::ContactList)
        .since(Timestamp::from(last_event_tt));

    user_contact_list
}

pub fn messages_filter(public_key: XOnlyPublicKey, last_event_tt: u64) -> Vec<Filter> {
    let sent_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .author(public_key.to_string())
        .since(Timestamp::from(last_event_tt));
    let recv_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .pubkey(public_key)
        .since(Timestamp::from(last_event_tt));

    vec![sent_msgs, recv_msgs]
}

pub fn channel_search_filter(channel_id: &str) -> Filter {
    // .search(search_term)
    // .hashtag(search_term)
    let mut channel_filter = Filter::new()
        .kind(Kind::ChannelCreation)
        .limit(CHANNEL_SEARCH_LIMIT);

    if !channel_id.is_empty() {
        channel_filter = channel_filter.id(channel_id);
    }

    channel_filter
}

pub fn channel_details_filter(channel_id: &nostr::EventId) -> Vec<Filter> {
    vec![
        Filter::new()
            .kind(Kind::ChannelMetadata)
            .event(channel_id.to_owned())
            .limit(10),
        Filter::new()
            .kind(Kind::ChannelMessage)
            .event(channel_id.to_owned())
            .limit(CHANNEL_DETAILS_LIMIT),
    ]
}
const CHANNEL_SEARCH_LIMIT: usize = 10;
const CHANNEL_DETAILS_LIMIT: usize = 1000;
