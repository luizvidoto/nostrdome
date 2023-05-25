use std::path::PathBuf;

use nostr::secp256k1::XOnlyPublicKey;

use crate::{
    components::chat_contact::ChatInfo,
    db::{DbContact, DbEvent, DbRelay, DbRelayResponse, ProfileCache},
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
    SubscribedToEvents,
    RequestedEventsOf(DbRelay),
    RequestedMetadata(DbContact),
    RequestedContactListProfiles,
    RequestedContactProfile(DbContact),
    GotChatInfo((DbContact, Option<ChatInfo>)),
    RelayMessage(nostr::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    ChannelCreated(nostr::EventId),
    NostrLoading,
    RequestedEvents,
    SentDirectMessage(nostr::EventId),
    ExportedMessagesSucessfully,
    ExportedContactsSucessfully,
    GotRelayStatus(ns_client::RelayStatus),
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
    RelaysConnected(usize),
}
impl std::fmt::Display for BackendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendEvent::GotRelayStatus(status) => write!(f, "Got relay status: {:?}", status),
            BackendEvent::RelaysConnected(count) => write!(f, "Relays connected: {}", count),
            BackendEvent::CacheFileRemoved((cache, kind)) => write!(
                f,
                "Cache file removed: {} - {}",
                cache.public_key,
                kind.to_str()
            ),
            BackendEvent::Empty => write!(f, "Empty"),
            BackendEvent::BackendLoading => write!(f, "Backend Loading"),
            BackendEvent::GotSingleContact((pubkey, ct)) => write!(
                f,
                "Got Single Contact: {} - is_some: {}",
                pubkey,
                ct.is_some()
            ),
            BackendEvent::GotDbEvent(db_event) => {
                write!(f, "Got DB Event: {:?}", db_event)
            }
            BackendEvent::GotRelayResponsesContactList { .. } => {
                write!(f, "Got Relay Responses Contact List")
            }
            BackendEvent::GotRelayResponsesUserProfile { .. } => {
                write!(f, "Got Relay Responses User Profile")
            }
            BackendEvent::GotChatInfo(_) => write!(f, "Got Chat Info"),
            BackendEvent::ExportedContactsSucessfully => write!(f, "Exported Contacts Sucessfully"),
            BackendEvent::ExportedMessagesSucessfully => write!(f, "Exported Messages Sucessfully"),
            BackendEvent::GotAllMessages(messages) => {
                write!(f, "Got All Messages: {}", messages.len())
            }
            BackendEvent::DownloadingImage { kind, public_key } => {
                write!(f, "Downloading Image: {} {}", kind.to_str(), public_key)
            }
            BackendEvent::UpdatedMetadata(pubkey) => write!(f, "Updated Metadata: {}", pubkey),
            BackendEvent::SystemTime(_) => write!(f, "System Time"),
            BackendEvent::SyncedWithNtpServer => write!(f, "Synced with NTP Server"),
            BackendEvent::SentEventTo((url, event_id)) => {
                write!(f, "Sent Event to: {}. ID: {}", url, event_id)
            }
            BackendEvent::SentEventToRelays(event_id) => {
                write!(f, "Sent Event to Relays: {}", event_id)
            }
            BackendEvent::PendingMetadata(_) => write!(f, "Pending Metadata"),
            BackendEvent::PendingContactList(_) => write!(f, "Pending Contact List"),
            BackendEvent::PendingDM(_) => write!(f, "Pending Direct Message"),
            BackendEvent::ConfirmedMetadata { .. } => write!(f, "Confirmed Metadata"),
            BackendEvent::ConfirmedContactList(_) => write!(f, "Confirmed Contact List"),
            BackendEvent::ConfirmedDM(_) => write!(f, "Confirmed Direct Message"),
            BackendEvent::RequestedContactProfile(db_contact) => write!(
                f,
                "Requested Contact Profile Public Key: {}",
                db_contact.pubkey()
            ),
            BackendEvent::SubscribedToEvents => write!(f, "Subscribed to Events"),
            BackendEvent::UserProfilePictureUpdated(_) => write!(f, "User Profile Picture Updated"),
            BackendEvent::UserBannerPictureUpdated(_) => write!(f, "User Banner Picture Updated"),
            BackendEvent::EndOfStoredEvents((relay_url, subscription_id)) => {
                write!(
                    f,
                    "End of stored events: {} --- Subscription ID: {}",
                    relay_url, subscription_id
                )
            }
            BackendEvent::RequestedMetadata(contact) => {
                write!(f, "Requested Metadata for public key: {}", contact.pubkey())
            }
            BackendEvent::RequestedContactListProfiles => {
                write!(f, "Requested Metadata for Contacts")
            }
            BackendEvent::RequestedEventsOf(db_relay) => {
                write!(f, "Requested events of: {}", db_relay.url)
            }
            BackendEvent::LatestVersion(version) => write!(f, "Latest Version: {}", version),
            BackendEvent::FetchingLatestVersion => write!(f, "Fetching Latest Version"),
            BackendEvent::ProfileCreated => write!(f, "Profile Created"),
            BackendEvent::OtherPendingEvent { .. } => write!(f, "Local Pending Event"),
            BackendEvent::GotChatMessages((contact, messages)) => {
                write!(
                    f,
                    "Got Chat Messages for contact public key: {:?}: {:?}",
                    contact.pubkey(),
                    messages.len()
                )
            }
            BackendEvent::GotRelayResponses { responses, .. } => {
                write!(f, "Got Relay Responses: {}", responses.len())
            }
            BackendEvent::GotContacts(contacts) => write!(f, "Got Contacts: {}", contacts.len()),
            BackendEvent::RelayCreated(db_relay) => write!(f, "Relay Created: {}", db_relay.url),
            BackendEvent::RelayUpdated(db_relay) => write!(f, "Relay Updated: {}", db_relay.url),
            BackendEvent::RelayDeleted(db_relay) => write!(f, "Relay Deleted: {}", db_relay.url),
            BackendEvent::GotRelays(relays) => write!(f, "Got Relays: {}", relays.len()),
            BackendEvent::ContactCreated(contact) => {
                write!(f, "Contact Created: {}", contact.pubkey())
            }
            BackendEvent::ContactUpdated(contact) => {
                write!(f, "Contact Updated: {}", contact.pubkey())
            }
            BackendEvent::ContactDeleted(contact) => {
                write!(f, "Contact Deleted: {}", contact.pubkey())
            }
            BackendEvent::OtherKindEventInserted(_) => write!(f, "Confirmed Event Inserted"),
            BackendEvent::FileContactsImported(contacts) => {
                write!(f, "File Contacts Imported: {}", contacts.len())
            }
            BackendEvent::GotUserProfileCache(metadata) => {
                write!(f, "Got User Profile Metadata: {:?}", metadata)
            }
            BackendEvent::RelayMessage(message) => write!(f, "Relay Message: {:?}", message),
            BackendEvent::Shutdown => write!(f, "Shutdown"),
            BackendEvent::RelayConnected(db_relay) => {
                write!(f, "Relay Connected: {}", db_relay.url)
            }
            BackendEvent::ChannelCreated(event_id) => {
                write!(f, "Channel Created: Event ID: {}", event_id)
            }
            BackendEvent::NostrLoading => write!(f, "Nostr Loading"),
            BackendEvent::RequestedEvents => write!(f, "Requested Events"),
            BackendEvent::SentDirectMessage(event_id) => {
                write!(f, "Sent Direct Message: Event ID: {}", event_id)
            }
            BackendEvent::FirstLogin => write!(f, "First Login"),
            BackendEvent::Connected(_) => write!(f, "Connected"),
            BackendEvent::Disconnected => write!(f, "Disconnected"),
            BackendEvent::FinishedPreparing => write!(f, "Finished Preparing"),
            BackendEvent::BackendClosed => write!(f, "Backend Closed"),
            BackendEvent::LoggedOut => write!(f, "Logged Out"),
            BackendEvent::Error(error) => write!(f, "Error: {}", error),
            BackendEvent::UpdatedUserProfileMeta { .. } => {
                write!(f, "Updated User Profile Metadata")
            }
            BackendEvent::UpdatedContactMetadata { db_contact, .. } => {
                write!(
                    f,
                    "Updated Contact Metadata: Contact public key: {}",
                    db_contact.pubkey()
                )
            }
            BackendEvent::ReceivedContactList { contact_list, .. } => {
                write!(f, "Relay Contacts Imported: {}", contact_list.len())
            }
            BackendEvent::ReceivedDM {
                db_contact,
                chat_message,
                ..
            } => {
                write!(
                    f,
                    "Received DM: Contact public key: {}, Message: {}",
                    db_contact.pubkey(),
                    chat_message.content
                )
            }
        }
    }
}
