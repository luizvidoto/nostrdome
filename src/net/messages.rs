use crate::{
    db::{DbContact, DbRelay},
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum Message {
    // -------- DATABASE MESSAGES
    QueryFirstLogin,
    StoreFirstLogin,
    PrepareClient,
    FetchRelayResponses(i64),
    FetchMessages(DbContact),
    FetchContacts,
    AddContact(DbContact),
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    ImportContacts(Vec<DbContact>),
    AddToUnseenCount(DbContact),
    FetchRelays,

    // -------- NOSTR CLIENT MESSAGES
    RequestEvents,
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    // UpdateRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay(DbRelay),
    BuildDM((DbContact, String)),
    SendDMToRelays(ChatMessage),
    SendContactListToRelay((DbRelay, Vec<DbContact>)),
    CreateChannel,
}
