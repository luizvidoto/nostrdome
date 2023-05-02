use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse},
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum DatabaseEvent {
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
}
