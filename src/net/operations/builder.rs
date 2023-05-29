use nostr::{Contact, EventBuilder, EventId, Keys, Metadata};
use sqlx::SqlitePool;
use thiserror::Error;

use crate::{
    db::{DbContact, UserConfig},
    utils::naive_to_event_tt,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Signing error: {0}")]
    SigningEventError(String),

    #[error("Nostr Sdk Event Builder Error: {0}")]
    NostrSdkEventBuilderError(#[from] nostr::prelude::builder::Error),
}

pub async fn build_profile_event(
    pool: &SqlitePool,
    keys: &Keys,
    metadata: &Metadata,
) -> Result<nostr::Event, Error> {
    tracing::debug!("send_profile");
    let builder = EventBuilder::set_metadata(metadata.clone());
    let ns_event = event_with_time(pool, keys, builder).await?;
    Ok(ns_event)
}

pub async fn build_contact_list_event(
    pool: &SqlitePool,
    keys: &Keys,
    list: &[DbContact],
) -> Result<nostr::Event, Error> {
    tracing::debug!("build_contact_list_event");
    let c_list: Vec<Contact> = list.iter().map(|c| c.into()).collect();
    let builder = EventBuilder::set_contact_list(c_list);
    let ns_event = event_with_time(pool, keys, builder).await?;
    Ok(ns_event)
}

pub async fn build_dm(
    pool: &SqlitePool,
    keys: &Keys,
    db_contact: &DbContact,
    content: &str,
) -> Result<nostr::Event, Error> {
    tracing::debug!("build_dm");
    let builder =
        EventBuilder::new_encrypted_direct_msg(&keys, db_contact.pubkey().to_owned(), content)?;
    let ns_event = event_with_time(pool, keys, builder).await?;
    Ok(ns_event)
}

async fn event_with_time(
    pool: &SqlitePool,
    keys: &Keys,
    builder: EventBuilder,
) -> Result<nostr::Event, Error> {
    let mut ns_event = builder.to_unsigned_event(keys.public_key());
    if let Ok(now_utc) = UserConfig::get_corrected_time(pool).await {
        ns_event.created_at = naive_to_event_tt(now_utc);
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
        .map_err(|e| Error::SigningEventError(e.to_string()))?;
    Ok(ns_event)
}
