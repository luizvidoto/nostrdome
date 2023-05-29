use thiserror::Error;

pub(crate) mod channel_cache;
pub(crate) mod contact;
pub(crate) mod database;
pub(crate) mod event;
pub(crate) mod message;
pub(crate) mod profile_cache;
pub(crate) mod relay;
pub(crate) mod relay_response;
pub(crate) mod user_config;

pub(crate) use channel_cache::ChannelCache;
pub(crate) use contact::DbContact;
pub(crate) use database::Database;
pub(crate) use event::DbEvent;
pub(crate) use message::{DbMessage, MessageStatus, TagInfo};
pub(crate) use profile_cache::ProfileCache;
pub(crate) use relay::DbRelay;
pub(crate) use relay_response::DbRelayResponse;
pub(crate) use user_config::UserConfig;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    ChannelCacheError(#[from] channel_cache::Error),

    #[error("{0}")]
    ContactError(#[from] contact::Error),

    #[error("{0}")]
    DatabaseError(#[from] database::Error),

    #[error("{0}")]
    EventError(#[from] event::Error),

    #[error("{0}")]
    MessageError(#[from] message::Error),

    #[error("{0}")]
    ProfileCacheError(#[from] profile_cache::Error),

    #[error("{0}")]
    RelayError(#[from] relay::Error),

    #[error("{0}")]
    RelayResponseError(#[from] relay_response::Error),

    #[error("{0}")]
    UserConfigError(#[from] user_config::Error),
}
