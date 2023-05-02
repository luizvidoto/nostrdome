use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse},
    types::ChatMessage,
};

pub(crate) mod backend;
// pub(crate) mod database;
// pub(crate) mod nostr;

#[derive(Debug, Clone)]
pub enum Event {
    BackEndEvent(backend::BackEndEvent),
    // --- Database ---
    LocalPendingEvent(DbEvent),
    DbConnected,
    DbDisconnected,
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
    EventReceived(DbEvent),
    // --- Nostr ---
    NostrConnected,
    NostrDisconnected,
    RelaysConnected,
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotRelayServers(Vec<nostr_sdk::Relay>),
    GotOwnEvents(Vec<nostr_sdk::Event>),
    GotPublicKey(XOnlyPublicKey),
    RelayMessage(nostr_sdk::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    UpdatedRelay,
    ChannelCreated(nostr_sdk::EventId),
    // --- Config ---
    IsProcessing,
    FinishedProcessing,
    Connected,
    Disconnected,
    FinishedPreparing,
    // --- General ---
    Error(String),
    None,
}

impl Event {
    pub fn backend(back: backend::BackEndEvent) -> Self {
        Self::BackEndEvent(back)
    }
}
