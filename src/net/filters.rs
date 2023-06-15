use nostr::{secp256k1::XOnlyPublicKey, Filter, Kind, Timestamp};

use crate::db::{DbContact, DbEvent};

fn to_secs(last_event: &Option<DbEvent>) -> u64 {
    last_event
        .as_ref()
        .map(|e| {
            // syncronization problems with different machines
            let earlier_time = e.created_at - chrono::Duration::minutes(10);
            (earlier_time.timestamp_millis() / 1000) as u64
        })
        .unwrap_or(0)
}

pub fn members_metadata_filter<'a, T: IntoIterator<Item = &'a XOnlyPublicKey>>(
    members: T,
) -> Filter {
    Filter::new()
        .authors(members.into_iter().map(|m| m.to_string()).collect())
        .kind(Kind::Metadata)
        .until(Timestamp::now())
}

pub fn contact_list_metadata_filter(
    contact_list: &[DbContact],
    last_event: &Option<DbEvent>,
) -> Option<Filter> {
    if contact_list.is_empty() {
        return None;
    }

    let all_pubkeys = contact_list
        .iter()
        .map(|c| c.pubkey().to_string())
        .collect::<Vec<_>>();

    Some(
        Filter::new()
            .authors(all_pubkeys)
            .kind(Kind::Metadata)
            .since(Timestamp::from(to_secs(last_event))),
    )
}

pub fn user_metadata_filter(pubkey: XOnlyPublicKey, last_event: &Option<DbEvent>) -> Filter {
    Filter::new()
        .author(pubkey.to_string())
        .kind(Kind::Metadata)
        .since(Timestamp::from(to_secs(last_event)))
}

pub fn contact_list_filter(public_key: XOnlyPublicKey, last_event: &Option<DbEvent>) -> Filter {
    let user_contact_list = Filter::new()
        .author(public_key.to_string())
        .kind(Kind::ContactList)
        .since(Timestamp::from(to_secs(last_event)));

    user_contact_list
}

pub fn messages_filter(public_key: XOnlyPublicKey, last_event: &Option<DbEvent>) -> Vec<Filter> {
    let sent_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .author(public_key.to_string())
        .since(Timestamp::from(to_secs(last_event)));
    let recv_msgs = Filter::new()
        .kind(nostr::Kind::EncryptedDirectMessage)
        .pubkey(public_key)
        .since(Timestamp::from(to_secs(last_event)));

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
            .until(Timestamp::now())
            .limit(10),
        Filter::new()
            .kind(Kind::ChannelMessage)
            .event(channel_id.to_owned())
            .until(Timestamp::now())
            .limit(CHANNEL_DETAILS_LIMIT),
    ]
}

pub fn subscribe_to_channels(
    channels: &[nostr::EventId],
    last_event: &Option<DbEvent>,
) -> Vec<Filter> {
    vec![
        Filter::new()
            .kind(Kind::ChannelMetadata)
            .events(channels.to_vec())
            .since(Timestamp::from(to_secs(last_event))),
        Filter::new()
            .kind(Kind::ChannelMessage)
            .events(channels.to_vec())
            .since(Timestamp::from(to_secs(last_event))),
        Filter::new()
            .kind(Kind::ChannelHideMessage)
            .events(channels.to_vec())
            .since(Timestamp::from(to_secs(last_event))),
        Filter::new()
            .kind(Kind::ChannelMuteUser)
            .events(channels.to_vec())
            .since(Timestamp::from(to_secs(last_event))),
    ]
}

const CHANNEL_SEARCH_LIMIT: usize = 10;
const CHANNEL_DETAILS_LIMIT: usize = 1000;
