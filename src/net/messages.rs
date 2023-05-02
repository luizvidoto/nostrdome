use crate::db::{DbContact, DbRelay};

#[derive(Debug, Clone)]
pub enum Message {
    // -------- DATABASE MESSAGES
    PrepareClient,
    ProcessMessages,
    FetchRelayResponses(i64),
    FetchMessages(DbContact),

    // Contacts
    FetchContacts,
    AddContact(DbContact),
    UpdateContact(DbContact),
    DeleteContact(DbContact),
    ImportContacts(Vec<DbContact>),
    AddToUnseenCount(DbContact),

    // Relays
    FetchRelays,
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    // UpdateRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),

    // -------- NOSTR CLIENT MESSAGES
    ConnectToRelay(DbRelay),
    SendDMTo((DbContact, String)),
    ShowPublicKey,
    SendContactListToRelay((DbRelay, Vec<DbContact>)),
    CreateChannel,
}
