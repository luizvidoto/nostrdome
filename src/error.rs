use futures::channel::mpsc;
use thiserror::Error;

/// Errors that can occur in the nostrtalk crate
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    FromConfig(#[from] crate::config::Error),

    #[error("{0}")]
    FromDbChannelMessage(#[from] crate::db::channel_message::Error),

    #[error("{0}")]
    FromChannelSubscription(#[from] crate::db::channel_subscription::Error),

    #[error("SendError: {0}")]
    FromSend(#[from] mpsc::SendError),

    // I/O Error
    #[error("I/O Error: {0}")]
    FromIo(#[from] std::io::Error),

    #[error("JSON (de)serialization error: {0}")]
    FromSerdeJson(#[from] serde_json::Error),

    #[error("Reqwest error: {0}")]
    FromReqwestClient(#[from] crate::net::reqwest_client::Error),

    #[error("Nostr Client Error: {0}")]
    FromNostrClient(#[from] ns_client::Error),

    #[error("Nostr Client Sdk Error: {0}")]
    FromNostrClientSdkError(#[from] nostr_sdk::client::Error),

    #[error("{0}")]
    FromBackendState(#[from] crate::types::backend_state::Error),

    #[error("{0}")]
    FromUtils(#[from] crate::utils::Error),

    #[error("{0}")]
    FromChannelMetadata(#[from] crate::types::channel_metadata::Error),

    #[error("{0}")]
    FromChatMessage(#[from] crate::types::chat_message::Error),

    #[error("{0}")]
    FromImageCache(#[from] crate::db::image_cache::Error),

    #[error("{0}")]
    FromChannelCache(#[from] crate::db::channel_cache::Error),

    #[error("{0}")]
    FromContact(#[from] crate::db::contact::Error),

    #[error("{0}")]
    FromDatabase(#[from] crate::db::database::Error),

    #[error("{0}")]
    FromEvent(#[from] crate::db::event::Error),

    #[error("{0}")]
    FromMessage(#[from] crate::db::message::Error),

    #[error("{0}")]
    FromProfileCache(#[from] crate::db::profile_cache::Error),

    #[error("{0}")]
    FromRelay(#[from] crate::db::relay::Error),

    #[error("{0}")]
    FromRelayResponse(#[from] crate::db::relay_response::Error),

    #[error("{0}")]
    FromUserConfig(#[from] crate::db::user_config::Error),

    #[error("{0}")]
    FromNtp(#[from] crate::net::ntp::NtpError),

    #[error("App didn't ask for kind: {0:?}")]
    NotSubscribedToKind(nostr::Kind),

    #[error("Not allowed to insert own pubkey as a contact")]
    SameContactInsert,

    #[error("Not allowed to update to own pubkey as a contact")]
    SameContactUpdate,

    #[error("{0}")]
    FromUrlParse(#[from] url::ParseError),

    #[error("Closed backend channel")]
    ClosedBackend(#[from] BackendClosed),

    #[error("Channel id not found in event tags: EventID: {0}")]
    ChannelIdNotFound(nostr::EventId),

    #[error("Unexpected event kind: {0}")]
    UnexpectedEventKind(u32),
}

#[derive(Error, Debug)]
pub struct BackendClosed;
impl std::fmt::Display for BackendClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Backend channel closed)")
    }
}
