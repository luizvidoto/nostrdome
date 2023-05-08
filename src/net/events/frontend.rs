use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse},
    net::BackEndConnection,
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum Event {
    // --- REQWEST ---
    LatestVersion(String),
    FetchingLatestVersion,
    // --- Database ---
    ProfileCreated,
    GotChatMessages((DbContact, Vec<ChatMessage>)),
    GotRelayResponses(Vec<DbRelayResponse>),
    GotContacts(Vec<DbContact>),
    RelayCreated(DbRelay),
    RelayUpdated(DbRelay),
    RelayDeleted(DbRelay),
    GotRelays(Vec<DbRelay>),
    ContactCreated(DbContact),
    ContactUpdated(DbContact),
    ContactDeleted(DbContact),
    EventInserted((DbEvent, Option<SpecificEvent>)),
    LocalPendingEvent((DbEvent, Option<SpecificEvent>)),
    // RelayEventDebugger((nostr_sdk::Url, Box<Event>)),
    UpdateWithRelayResponse {
        relay_response: DbRelayResponse,
        db_event: DbEvent,
        db_message: Option<DbMessage>,
    },
    GotUserProfileMeta(nostr_sdk::Metadata),
    FileContactsImported(Vec<DbContact>),
    // --- Nostr ---
    EndOfStoredEvents((nostr_sdk::Url, nostr_sdk::SubscriptionId)),
    RequestedEventsOf(DbRelay),
    RequestedMetadata(DbContact),
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotRelayServers(Vec<nostr_sdk::Relay>),
    RelayMessage(nostr_sdk::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    ChannelCreated(nostr_sdk::EventId),
    NostrLoading,
    RequestedEvents,
    SentDirectMessage(nostr_sdk::EventId),
    // --- Config ---
    FirstLogin,
    Connected(BackEndConnection),
    Disconnected,
    FirstLoginStored,
    FinishedPreparing,
    // --- General ---
    BackendClosed,
    LoggedOut,
    Error(String),
    None,
}
impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::EndOfStoredEvents((relay_url, subscription_id)) => {
                write!(
                    f,
                    "End of stored events: {} --- Subscription ID: {}",
                    relay_url, subscription_id
                )
            }
            Event::RequestedMetadata(contact) => {
                write!(f, "Requested Metadata for public key: {}", contact.pubkey())
            }

            Event::RequestedEventsOf(db_relay) => {
                write!(f, "Requested events of: {}", db_relay.url)
            }
            Event::LatestVersion(version) => write!(f, "Latest Version: {}", version),
            Event::FetchingLatestVersion => write!(f, "Fetching Latest Version"),
            Event::ProfileCreated => write!(f, "Profile Created"),
            Event::LocalPendingEvent(_) => write!(f, "Local Pending Event"),
            Event::GotChatMessages((contact, messages)) => {
                write!(
                    f,
                    "Got Chat Messages for contact public key: {:?}: {:?}",
                    contact.pubkey(),
                    messages.len()
                )
            }
            Event::GotRelayResponses(responses) => {
                write!(f, "Got Relay Responses: {}", responses.len())
            }
            Event::GotContacts(contacts) => write!(f, "Got Contacts: {}", contacts.len()),
            Event::RelayCreated(db_relay) => write!(f, "Relay Created: {}", db_relay.url),
            Event::RelayUpdated(db_relay) => write!(f, "Relay Updated: {}", db_relay.url),
            Event::RelayDeleted(db_relay) => write!(f, "Relay Deleted: {}", db_relay.url),
            Event::GotRelays(relays) => write!(f, "Got Relays: {}", relays.len()),
            Event::ContactCreated(contact) => write!(f, "Contact Created: {}", contact.pubkey()),
            Event::ContactUpdated(contact) => write!(f, "Contact Updated: {}", contact.pubkey()),
            Event::ContactDeleted(contact) => write!(f, "Contact Deleted: {}", contact.pubkey()),
            Event::EventInserted(_) => write!(f, "Event Inserted"),
            Event::UpdateWithRelayResponse { relay_response, .. } => write!(
                f,
                "Update With Relay Response: {}",
                relay_response.relay_url
            ),
            Event::FileContactsImported(contacts) => {
                write!(f, "File Contacts Imported: {}", contacts.len())
            }
            Event::GotUserProfileMeta(metadata) => {
                write!(f, "Got User Profile Metadata: {:?}", metadata)
            }
            Event::GotRelayServer(_server) => write!(f, "Got Relay Server"),
            Event::GotRelayServers(_servers) => write!(f, "Got Relay Servers"),
            Event::RelayMessage(message) => write!(f, "Relay Message: {:?}", message),
            Event::Shutdown => write!(f, "Shutdown"),
            Event::RelayConnected(db_relay) => write!(f, "Relay Connected: {}", db_relay.url),
            Event::ChannelCreated(event_id) => {
                write!(f, "Channel Created: Event ID: {}", event_id)
            }
            Event::NostrLoading => write!(f, "Nostr Loading"),
            Event::RequestedEvents => write!(f, "Requested Events"),
            Event::SentDirectMessage(event_id) => {
                write!(f, "Sent Direct Message: Event ID: {}", event_id)
            }
            Event::FirstLogin => write!(f, "First Login"),
            Event::Connected(_) => write!(f, "Connected"),
            Event::Disconnected => write!(f, "Disconnected"),
            Event::FirstLoginStored => write!(f, "First Login Stored"),
            Event::FinishedPreparing => write!(f, "Finished Preparing"),
            Event::BackendClosed => write!(f, "Backend Closed"),
            Event::LoggedOut => write!(f, "Logged Out"),
            Event::Error(error) => write!(f, "Error: {}", error),
            Event::None => write!(f, "None"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SpecificEvent {
    ReceivedDM((DbContact, ChatMessage)),
    NewDMAndContact((DbContact, ChatMessage)),
    RelayContactsImported(Vec<DbContact>),
    UpdatedContactMetadata((DbContact, nostr_sdk::Metadata)),
    UpdatedUserProfileMeta(nostr_sdk::Metadata),
}

impl std::fmt::Display for SpecificEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecificEvent::UpdatedUserProfileMeta(_metadata) => {
                write!(f, "Updated User Profile Metadata")
            }
            SpecificEvent::UpdatedContactMetadata((contact, metadata)) => {
                write!(
                    f,
                    "Updated Contact Metadata: Contact public key: {}, Metadata: {:?}",
                    contact.pubkey(),
                    metadata
                )
            }
            SpecificEvent::RelayContactsImported(contacts) => {
                write!(f, "Relay Contacts Imported: {}", contacts.len())
            }
            SpecificEvent::ReceivedDM((contact, message)) => {
                write!(
                    f,
                    "Received DM: Contact public key: {}, Message: {}",
                    contact.pubkey(),
                    message.content
                )
            }
            SpecificEvent::NewDMAndContact((contact, message)) => {
                write!(
                    f,
                    "New DM and Contact: Contact public key: {}, Message: {}",
                    contact.pubkey(),
                    message.content
                )
            }
        }
    }
}
