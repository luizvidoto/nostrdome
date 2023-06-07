use nostr::SubscriptionId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrefixedId(String);
impl PrefixedId {
    pub fn new(id: &str) -> Self {
        if id.len() >= 8 {
            Self(id[0..8].to_owned())
        } else {
            Self(id.to_owned())
        }
    }
}
impl std::fmt::Display for PrefixedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}
#[derive(Clone)]
pub enum SubscriptionType {
    ContactList,
    ContactListMetadata,
    UserMetadata,
    Messages,
    SearchChannels,
    SearchChannelsDetails(PrefixedId),
}
impl SubscriptionType {
    pub fn src_channel_details(channel_id: &nostr::EventId) -> Self {
        Self::SearchChannelsDetails(PrefixedId::new(&channel_id.to_hex()))
    }
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
                    Some(SubscriptionType::SearchChannelsDetails(PrefixedId(
                        hex.to_owned(),
                    )))
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
            SubscriptionType::SearchChannelsDetails(prefixed) => {
                write!(f, "SrcChannelDts_{}", &prefixed)
            }
        }
    }
}
impl SubscriptionType {
    pub fn id(&self) -> SubscriptionId {
        match self {
            SubscriptionType::SearchChannelsDetails(channel_id) => {
                SubscriptionId::new(format!("SrcChannelDts_{}", &channel_id))
            }
            other => SubscriptionId::new(other.to_string()),
        }
    }
}
