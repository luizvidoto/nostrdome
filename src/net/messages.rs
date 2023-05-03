use crate::db::{DbContact, DbRelay};

#[derive(Debug, Clone)]
pub enum Message {
    // -------- DATABASE MESSAGES
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
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    // UpdateRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay(DbRelay),
    SendDMTo((DbContact, String)),
    SendContactListToRelay((DbRelay, Vec<DbContact>)),
    CreateChannel,
}
