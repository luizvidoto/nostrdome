use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse},
    net::BackEndConnection,
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum Event {
    // --- Database ---
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
    // --- Nostr ---
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotRelayServers(Vec<nostr_sdk::Relay>),
    RelayMessage(nostr_sdk::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    ChannelCreated(nostr_sdk::EventId),
    NostrLoading,
    // --- Config ---
    Connected(BackEndConnection),
    Disconnected,
    FinishedPreparing,
    // --- General ---
    Error(String),
    None,
}
