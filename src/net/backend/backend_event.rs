use std::path::PathBuf;

use nostr::secp256k1::XOnlyPublicKey;

use crate::{
    components::chat_contact::ChatInfo,
    db::{ChannelCache, DbContact, DbEvent, DbRelay, DbRelayResponse, ProfileCache},
    net::{BackEndConnection, ImageKind},
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum BackendEvent {
    // --- REQWEST ---
    LatestVersion(String),
    FetchingLatestVersion,
    DownloadingImage {
        kind: ImageKind,
        public_key: XOnlyPublicKey,
    },
    // --- Database ---
    ProfileCreated,
    GotChatMessages((DbContact, Vec<ChatMessage>)),
    GotRelayResponses {
        chat_message: ChatMessage,
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotRelayResponsesUserProfile {
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotRelayResponsesContactList {
        responses: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    },
    GotContacts(Vec<DbContact>),
    RelayCreated(DbRelay),
    RelayUpdated(DbRelay),
    RelayDeleted(DbRelay),
    GotRelays(Vec<DbRelay>),
    ContactCreated(DbContact),
    ContactUpdated(DbContact),
    ContactDeleted(DbContact),
    OtherKindEventInserted(DbEvent),
    GotUserProfileCache(Option<ProfileCache>),
    FileContactsImported(Vec<DbContact>),
    UserProfilePictureUpdated(PathBuf),
    UserBannerPictureUpdated(PathBuf),
    SystemTime((chrono::NaiveDateTime, i64)),
    UpdatedMetadata(XOnlyPublicKey),
    GotAllMessages(Vec<DbEvent>),
    GotDbEvent(Option<DbEvent>),
    GotSingleContact((XOnlyPublicKey, Option<DbContact>)),

    // --- Nostr ---
    SentEventToRelays(nostr::EventId),
    SentEventTo((url::Url, nostr::EventId)),
    EndOfStoredEvents((nostr::Url, nostr::SubscriptionId)),
    RequestedEventsOf(DbRelay),
    RequestedMetadata(DbContact),
    RequestedContactListProfiles,
    RequestedContactProfile(DbContact),
    GotChatInfo((DbContact, Option<ChatInfo>)),
    RelayMessage(nostr::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    ChannelCreated(ChannelCache),
    NostrLoading,
    RequestedEvents,
    SentDirectMessage(nostr::EventId),
    ExportedMessagesSucessfully,
    ExportedContactsSucessfully,
    GotRelayStatus((nostr::Url, ns_client::RelayStatus)),
    GotRelayStatusList(ns_client::RelayStatusList),
    // --- Config ---
    SyncedWithNtpServer,
    FirstLogin,
    Connected(BackEndConnection),
    Disconnected,
    FinishedPreparing,
    // --- Specific Events ---
    ReceivedDM {
        relay_url: nostr::Url,
        db_contact: DbContact,
        chat_message: ChatMessage,
    },
    ReceivedContactList {
        relay_url: nostr::Url,
        contact_list: Vec<DbContact>,
    },
    UpdatedContactMetadata {
        relay_url: nostr::Url,
        db_contact: DbContact,
    },
    UpdatedUserProfileMeta {
        relay_url: nostr::Url,
        metadata: nostr::Metadata,
    },
    // --- Confirmed Events ---
    ConfirmedDM((DbContact, ChatMessage)),
    ConfirmedContactList(DbEvent),
    ConfirmedMetadata {
        db_event: DbEvent,
        is_user: bool,
    },

    // --- Pending Events ---
    OtherPendingEvent(DbEvent),
    PendingDM((DbContact, ChatMessage)),
    PendingContactList(DbEvent),
    PendingMetadata(DbEvent),
    // --- General ---
    BackendClosed,
    LoggedOut,
    Error(String),
    BackendLoading,
    Empty,
    CacheFileRemoved((ProfileCache, ImageKind)),
    ChannelUpdated(ChannelCache),
    ChoosenFile(PathBuf),
    ExportedContactsToIdle,
    ExportedMessagesToIdle,
}
