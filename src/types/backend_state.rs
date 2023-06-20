use std::collections::HashMap;

use chrono::NaiveDateTime;
use nostr::{Contact, EventBuilder, EventId, Keys, Metadata, Timestamp};
use ns_client::RelayPool;
use sqlx::SqlitePool;
use thiserror::Error;
use url::Url;

use crate::{
    db::{Database, DbContact, UserConfig},
    net::ntp::system_now_microseconds,
    utils::{
        channel_creation_builder, channel_metadata_builder, channel_msg_builder, naive_to_event_tt,
        ns_event_to_naive, NipData,
    },
    views::login::BasicProfile,
};

use super::ChannelMetadata;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Signing error: {0}")]
    SigningEvent(String),

    #[error("Nostr Sdk Event Builder Error: {0}")]
    NostrSdkEventBuilder(#[from] nostr::prelude::builder::Error),

    #[error("Nostr NsClient Error: {0}")]
    FromNsClient(#[from] ns_client::Error),

    #[error("{0}")]
    FromDbContact(#[from] crate::db::contact::Error),

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(Timestamp),
}

#[derive(Debug, Clone)]
pub struct PendingEvent(nostr::Event);
impl PendingEvent {
    fn new(ns_event: nostr::Event) -> Self {
        Self(ns_event)
    }
    pub fn id(&self) -> &EventId {
        &self.0.id
    }
    pub fn ns_event(&self) -> &nostr::Event {
        &self.0
    }
    pub fn event_hash(&self) -> EventId {
        self.0.id.to_owned()
    }
    pub fn display_time(&self) -> Result<NaiveDateTime, Error> {
        ns_event_to_naive(self.0.created_at).map_err(|_| Error::InvalidTimestamp(self.0.created_at))
    }
}

pub struct BackendState {
    pub req_client: reqwest::Client,
    pub nostr: RelayPool,
    pub nips_data: Vec<NipData>,
    pub create_account: Option<BasicProfile>,
    pub pending_events: HashMap<EventId, PendingEvent>,
    db_client: Database,
    ntp_offset: Option<i64>,
    ntp_server: Option<String>,
}
impl BackendState {
    pub fn new(
        db_client: Database,
        req_client: reqwest::Client,
        nostr: RelayPool,
        nips_data: Vec<NipData>,
        create_account: Option<BasicProfile>,
    ) -> Self {
        Self {
            db_client,
            req_client,
            nostr,
            nips_data,
            create_account,
            pending_events: HashMap::new(),
            ntp_offset: None,
            ntp_server: None,
        }
    }

    fn insert_pending(&mut self, event: PendingEvent) {
        self.pending_events.insert(*event.id(), event);
    }
    pub fn synced_ntp(&self) -> (Option<i64>, Option<String>) {
        (self.ntp_offset, self.ntp_server.clone())
    }
    pub fn update_ntp(&mut self, ntp_time: u64, server: &str) {
        let system_microseconds = system_now_microseconds().expect("System time before unix epoch");
        let offset = ntp_time as i64 - system_microseconds as i64;

        self.ntp_offset = Some(offset);
        self.ntp_server = Some(server.to_owned());
    }
    pub async fn new_auth_event<S>(
        &mut self,
        keys: &Keys,
        relay_url: &Url,
        challenge: S,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        tracing::debug!("send_auth");
        let pool = &self.db_client.pool;

        let builder = EventBuilder::auth(challenge, relay_url.to_owned());
        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_auth(relay_url, ns_event)?;
        Ok(())
    }

    pub async fn new_profile_event(
        &mut self,
        keys: &Keys,
        metadata: &Metadata,
    ) -> Result<(), Error> {
        tracing::debug!("send_profile");
        let pool = &self.db_client.pool;

        let builder = EventBuilder::set_metadata(metadata.clone());
        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        self.insert_pending(PendingEvent::new(ns_event));

        Ok(())
    }

    pub async fn new_contact_list_event(&mut self, keys: &Keys) -> Result<PendingEvent, Error> {
        tracing::debug!("build_contact_list_event");
        let pool = &self.db_client.pool;
        let list = DbContact::fetch_basic(&self.db_client.pool).await?;
        let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();

        let builder = EventBuilder::set_contact_list(c_list);
        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        let pending_event = PendingEvent::new(ns_event);
        self.insert_pending(pending_event.clone());

        Ok(pending_event)
    }

    pub async fn new_dm(
        &mut self,
        keys: &Keys,
        db_contact: &DbContact,
        content: &str,
    ) -> Result<PendingEvent, Error> {
        tracing::debug!("build_dm");
        let pool = &self.db_client.pool;

        let builder =
            EventBuilder::new_encrypted_direct_msg(keys, db_contact.pubkey().to_owned(), content)?;
        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        let pending_event = PendingEvent::new(ns_event);
        self.insert_pending(pending_event.clone());

        Ok(pending_event)
    }

    pub(crate) async fn new_channel_msg(
        &mut self,
        keys: &Keys,
        channel_id: &EventId,
        recommended_relay: Option<&Url>,
        content: &str,
    ) -> Result<PendingEvent, Error> {
        let pool = &self.db_client.pool;
        let builder = channel_msg_builder(channel_id, recommended_relay, content);

        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        let pending_event = PendingEvent::new(ns_event);
        self.insert_pending(pending_event.clone());

        Ok(pending_event)
    }

    pub(crate) async fn new_channel(
        &mut self,
        keys: &Keys,
        metadata: &ChannelMetadata,
    ) -> Result<PendingEvent, Error> {
        let pool = &self.db_client.pool;
        let builder = channel_creation_builder(metadata);

        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        let pending_event = PendingEvent::new(ns_event);
        self.insert_pending(pending_event.clone());

        Ok(pending_event)
    }

    pub(crate) async fn new_channel_metadata(
        &mut self,
        keys: &Keys,
        channel_id: &EventId,
        recommended_relay: Option<&Url>,
        metadata: &ChannelMetadata,
    ) -> Result<PendingEvent, Error> {
        let pool = &self.db_client.pool;
        let builder = channel_metadata_builder(channel_id, recommended_relay, metadata);

        let ns_event = event_with_time(pool, keys, builder).await?;
        self.nostr.send_event(ns_event.clone())?;

        let pending_event = PendingEvent::new(ns_event);
        self.insert_pending(pending_event.clone());

        Ok(pending_event)
    }

    pub async fn logout(&self) -> Result<(), Error> {
        tracing::info!("Database Logging out");
        self.db_client.pool.close().await;
        self.db_client.cache_pool.close().await;
        self.nostr.shutdown()?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.db_client.pool
    }

    pub fn cache_pool(&self) -> &SqlitePool {
        &self.db_client.cache_pool
    }
}

async fn event_with_time(
    pool: &SqlitePool,
    keys: &Keys,
    builder: EventBuilder,
) -> Result<nostr::Event, Error> {
    let mut ns_event = builder.to_unsigned_event(keys.public_key());
    if let Ok(utc_now) = UserConfig::get_corrected_time(pool).await {
        ns_event.created_at = naive_to_event_tt(utc_now);
    }
    let updated_id = EventId::new(
        &keys.public_key(),
        ns_event.created_at,
        &ns_event.kind,
        &ns_event.tags,
        &ns_event.content,
    );
    ns_event.id = updated_id;
    let ns_event = ns_event
        .sign(keys)
        .map_err(|e| Error::SigningEvent(e.to_string()))?;
    Ok(ns_event)
}
