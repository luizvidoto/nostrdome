use crate::{
    db::{DbContact, DbRelay},
    types::ChatMessage,
    views::login::BasicProfile,
};

#[derive(Debug, Clone)]
pub enum Message {
    Logout,
    // -------- REQWEST MESSAGES
    FetchLatestVersion,
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
    ImportContacts((Vec<DbContact>, bool)),
    AddToUnseenCount(DbContact),
    FetchRelays,
    CreateAccount(BasicProfile),
    GetUserProfileMeta,
    UpdateUserProfileMeta(nostr_sdk::Metadata),

    // -------- NOSTR CLIENT MESSAGES
    RequestEventsOf(DbRelay),
    RefreshContactsProfile,
    SubscribeToEvents,
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay(DbRelay),
    BuildDM((DbContact, String)),
    SendDMToRelays(ChatMessage),
    SendContactListToRelay(DbRelay),
    CreateChannel,
}
