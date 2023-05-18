use std::path::PathBuf;

use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::{
    db::{DbContact, DbEvent, DbRelay},
    types::ChatMessage,
    views::login::BasicProfile,
};

use super::ImageKind;

#[derive(Debug, Clone)]
pub enum Message {
    Logout,
    // -------- REQWEST MESSAGES
    FetchLatestVersion,
    // -------- DATABASE MESSAGES
    QueryFirstLogin,
    StoreFirstLogin,
    PrepareClient,
    FetchRelayResponses(ChatMessage),
    FetchRelayResponsesUserProfile,
    FetchRelayResponsesContactList,
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
    FetchAllMessageEvents,
    ExportMessages((Vec<DbEvent>, PathBuf)),
    ExportContacts(std::path::PathBuf),
    FetchLastChatMessage(DbContact),
    GetDbEventWithHash(nostr_sdk::EventId),

    // -------- NOSTR CLIENT MESSAGES
    RequestEventsOf(DbRelay),
    RefreshContactsProfile,
    FetchRelayServer(nostr_sdk::Url),
    FetchRelayServers,
    AddRelay(DbRelay),
    DeleteRelay(DbRelay),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    ConnectToRelay(DbRelay),
    SendDM((DbContact, String)),
    SendContactListToRelays,
    CreateChannel,
    DownloadImage {
        image_url: String,
        kind: ImageKind,
        public_key: XOnlyPublicKey,
    },
}
