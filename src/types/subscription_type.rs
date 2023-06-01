use nostr::{EventId, SubscriptionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    ContactList,
    ContactListMetadata,
    UserMetadata,
    Messages,
    SearchChannels,
    SearchChannelsDetails(EventId),
}
impl SubscriptionType {
    pub fn from_id(id: &SubscriptionId) -> Option<Self> {
        let str = id.to_string();
        match str.as_str() {
            "ContactList" => Some(SubscriptionType::ContactList),
            "ContactListMetadata" => Some(SubscriptionType::ContactListMetadata),
            "UserMetadata" => Some(SubscriptionType::UserMetadata),
            "Messages" => Some(SubscriptionType::Messages),
            "SearchChannels" => Some(SubscriptionType::SearchChannels),
            _ => {
                if str.starts_with("SrcChannelDts_") {
                    let (_, hex) = str.split_at("SrcChannelDts_".len());
                    match EventId::from_hex(hex) {
                        Ok(event_id) => Some(SubscriptionType::SearchChannelsDetails(event_id)),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
        }
    }
}

impl std::fmt::Display for SubscriptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionType::ContactList => write!(f, "ContactList"),
            SubscriptionType::ContactListMetadata => write!(f, "ContactListMetadata"),
            SubscriptionType::UserMetadata => write!(f, "UserMetadata"),
            SubscriptionType::Messages => write!(f, "Messages"),
            SubscriptionType::SearchChannels => write!(f, "SearchChannels"),
            SubscriptionType::SearchChannelsDetails(channel_id) => {
                write!(f, "SrcChannelDts_{}", &channel_id.to_hex()[0..8])
            }
        }
    }
}
impl SubscriptionType {
    pub fn id(&self) -> SubscriptionId {
        match self {
            SubscriptionType::SearchChannelsDetails(channel_id) => {
                SubscriptionId::new(format!("SrcChannelDts_{}", &channel_id.to_hex()[0..8]))
            }
            other => SubscriptionId::new(other.to_string()),
        }
    }
}
