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
    LocalPendingEvent(DbEvent),
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
    ContactsImported(Vec<DbContact>),
    EventInserted(DbEvent),
    ReceivedDM((DbContact, ChatMessage)),
    NewDMAndContact((DbContact, ChatMessage)),
    UpdateWithRelayResponse {
        relay_response: DbRelayResponse,
        db_event: DbEvent,
        db_message: Option<DbMessage>,
    },
    GotUserProfileMeta(nostr_sdk::Metadata),
    UpdatedUserProfileMeta(nostr_sdk::Metadata),
    // --- Nostr ---
    SentProfileMeta((nostr_sdk::Metadata, nostr_sdk::EventId)),
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotRelayServers(Vec<nostr_sdk::Relay>),
    RelayMessage(nostr_sdk::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    ChannelCreated(nostr_sdk::EventId),
    NostrLoading,
    RequestedEvents,
    SentDirectMessage(nostr_sdk::EventId),
    RelayEventsUpdated(nostr_sdk::Url),
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
