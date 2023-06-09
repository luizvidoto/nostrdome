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
pub enum SubName {
    ContactList,
    ContactListMetadata,
    UserMetadata,
    Messages,
    SearchChannels,
    SearchChannelsDetails(PrefixedId),
}
impl SubName {
    pub fn src_channel_details(channel_id: &nostr::EventId) -> Self {
        Self::SearchChannelsDetails(PrefixedId::new(&channel_id.to_hex()))
    }
    pub fn from_id(id: &SubscriptionId) -> Option<Self> {
        let str = id.to_string();
        match str.as_str() {
            "ContactList" => Some(SubName::ContactList),
            "ContactListMetadata" => Some(SubName::ContactListMetadata),
            "UserMetadata" => Some(SubName::UserMetadata),
            "Messages" => Some(SubName::Messages),
            "SearchChannels" => Some(SubName::SearchChannels),
            _ => {
                if str.starts_with("SrcChannelDts_") {
                    let (_, hex) = str.split_at("SrcChannelDts_".len());
                    Some(SubName::SearchChannelsDetails(PrefixedId(hex.to_owned())))
                } else {
                    None
                }
            }
        }
    }
}

impl std::fmt::Display for SubName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubName::ContactList => write!(f, "ContactList"),
            SubName::ContactListMetadata => write!(f, "ContactListMetadata"),
            SubName::UserMetadata => write!(f, "UserMetadata"),
            SubName::Messages => write!(f, "Messages"),
            SubName::SearchChannels => write!(f, "SearchChannels"),
            SubName::SearchChannelsDetails(prefixed) => {
                write!(f, "SrcChannelDts_{}", &prefixed)
            }
        }
    }
}
