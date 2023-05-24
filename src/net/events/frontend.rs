use std::path::PathBuf;

use nostr::secp256k1::XOnlyPublicKey;

use crate::{
    components::chat_contact::ChatInfo,
    db::{DbContact, DbEvent, DbRelay, DbRelayResponse, ProfileCache},
    net::{BackEndConnection, ImageKind},
    types::ChatMessage,
};

#[derive(Debug, Clone)]
pub enum Event {
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
    GotDbEvent(DbEvent),
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
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotRelayServers(Vec<nostr_sdk::Relay>),
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
    None,
}
impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::GotSingleContact((pubkey, ct)) => write!(
                f,
                "Got Single Contact: {} - is_some: {}",
                pubkey,
                ct.is_some()
            ),
            Event::GotDbEvent(db_event) => write!(f, "Got DB Event: {}", db_event.event_hash),
            Event::GotRelayResponsesContactList { .. } => {
                write!(f, "Got Relay Responses Contact List")
            }
            Event::GotRelayResponsesUserProfile { .. } => {
                write!(f, "Got Relay Responses User Profile")
            }
            Event::GotChatInfo(_) => write!(f, "Got Chat Info"),
            Event::ExportedContactsSucessfully => write!(f, "Exported Contacts Sucessfully"),
            Event::ExportedMessagesSucessfully => write!(f, "Exported Messages Sucessfully"),
            Event::GotAllMessages(messages) => write!(f, "Got All Messages: {}", messages.len()),
            Event::DownloadingImage { kind, public_key } => {
                write!(f, "Downloading Image: {} {}", kind.to_str(), public_key)
            }
            Event::UpdatedMetadata(pubkey) => write!(f, "Updated Metadata: {}", pubkey),
            Event::SystemTime(_) => write!(f, "System Time"),
            Event::SyncedWithNtpServer => write!(f, "Synced with NTP Server"),
            Event::SentEventTo((url, event_id)) => {
                write!(f, "Sent Event to: {}. ID: {}", url, event_id)
            }
            Event::SentEventToRelays(event_id) => write!(f, "Sent Event to Relays: {}", event_id),
            Event::PendingMetadata(_) => write!(f, "Pending Metadata"),
            Event::PendingContactList(_) => write!(f, "Pending Contact List"),
            Event::PendingDM(_) => write!(f, "Pending Direct Message"),
            Event::ConfirmedMetadata { .. } => write!(f, "Confirmed Metadata"),
            Event::ConfirmedContactList(_) => write!(f, "Confirmed Contact List"),
            Event::ConfirmedDM(_) => write!(f, "Confirmed Direct Message"),
            Event::RequestedContactProfile(db_contact) => write!(
                f,
                "Requested Contact Profile Public Key: {}",
                db_contact.pubkey()
            ),
            Event::SubscribedToEvents => write!(f, "Subscribed to Events"),
            Event::UserProfilePictureUpdated(_) => write!(f, "User Profile Picture Updated"),
            Event::UserBannerPictureUpdated(_) => write!(f, "User Banner Picture Updated"),
            Event::EndOfStoredEvents((relay_url, subscription_id)) => {
                write!(
                    f,
                    "End of stored events: {} --- Subscription ID: {}",
                    relay_url, subscription_id
                )
            }
            Event::RequestedMetadata(contact) => {
                write!(f, "Requested Metadata for public key: {}", contact.pubkey())
            }
            Event::RequestedContactListProfiles => write!(f, "Requested Metadata for Contacts"),
            Event::RequestedEventsOf(db_relay) => {
                write!(f, "Requested events of: {}", db_relay.url)
            }
            Event::LatestVersion(version) => write!(f, "Latest Version: {}", version),
            Event::FetchingLatestVersion => write!(f, "Fetching Latest Version"),
            Event::ProfileCreated => write!(f, "Profile Created"),
            Event::OtherPendingEvent { .. } => write!(f, "Local Pending Event"),
            Event::GotChatMessages((contact, messages)) => {
                write!(
                    f,
                    "Got Chat Messages for contact public key: {:?}: {:?}",
                    contact.pubkey(),
                    messages.len()
                )
            }
            Event::GotRelayResponses { responses, .. } => {
                write!(f, "Got Relay Responses: {}", responses.len())
            }
            Event::GotContacts(contacts) => write!(f, "Got Contacts: {}", contacts.len()),
            Event::RelayCreated(db_relay) => write!(f, "Relay Created: {}", db_relay.url),
            Event::RelayUpdated(db_relay) => write!(f, "Relay Updated: {}", db_relay.url),
            Event::RelayDeleted(db_relay) => write!(f, "Relay Deleted: {}", db_relay.url),
            Event::GotRelays(relays) => write!(f, "Got Relays: {}", relays.len()),
            Event::ContactCreated(contact) => write!(f, "Contact Created: {}", contact.pubkey()),
            Event::ContactUpdated(contact) => write!(f, "Contact Updated: {}", contact.pubkey()),
            Event::ContactDeleted(contact) => write!(f, "Contact Deleted: {}", contact.pubkey()),
            Event::OtherKindEventInserted(_) => write!(f, "Confirmed Event Inserted"),
            Event::FileContactsImported(contacts) => {
                write!(f, "File Contacts Imported: {}", contacts.len())
            }
            Event::GotUserProfileCache(metadata) => {
                write!(f, "Got User Profile Metadata: {:?}", metadata)
            }
            Event::GotRelayServer(_server) => write!(f, "Got Relay Server"),
            Event::GotRelayServers(_servers) => write!(f, "Got Relay Servers"),
            Event::RelayMessage(message) => write!(f, "Relay Message: {:?}", message),
            Event::Shutdown => write!(f, "Shutdown"),
            Event::RelayConnected(db_relay) => write!(f, "Relay Connected: {}", db_relay.url),
            Event::ChannelCreated(event_id) => {
                write!(f, "Channel Created: Event ID: {}", event_id)
            }
            Event::NostrLoading => write!(f, "Nostr Loading"),
            Event::RequestedEvents => write!(f, "Requested Events"),
            Event::SentDirectMessage(event_id) => {
                write!(f, "Sent Direct Message: Event ID: {}", event_id)
            }
            Event::FirstLogin => write!(f, "First Login"),
            Event::Connected(_) => write!(f, "Connected"),
            Event::Disconnected => write!(f, "Disconnected"),
            Event::FinishedPreparing => write!(f, "Finished Preparing"),
            Event::BackendClosed => write!(f, "Backend Closed"),
            Event::LoggedOut => write!(f, "Logged Out"),
            Event::Error(error) => write!(f, "Error: {}", error),
            Event::UpdatedUserProfileMeta { .. } => {
                write!(f, "Updated User Profile Metadata")
            }
            Event::UpdatedContactMetadata { db_contact, .. } => {
                write!(
                    f,
                    "Updated Contact Metadata: Contact public key: {}",
                    db_contact.pubkey()
                )
            }
            Event::ReceivedContactList { contact_list, .. } => {
                write!(f, "Relay Contacts Imported: {}", contact_list.len())
            }
            Event::ReceivedDM {
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
            Event::None => write!(f, "None"),
        }
    }
}
