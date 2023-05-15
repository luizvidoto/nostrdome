use std::path::PathBuf;

use chrono::{NaiveDateTime, Utc};
use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys, Metadata, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, UserConfig},
    error::Error,
    net::{
        events::{backend::BackEndInput, Event},
        to_backend_channel,
    },
};

use super::builder::build_profile_event;

// Handle metadata events and update user profile or contact metadata accordingly.
pub async fn handle_metadata_event(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("handle_metadata_event");

    // create event struct
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    // insert into database
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::info!("Event already in database");
        return Ok(Event::None);
    }

    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::debug!("{:?}", db_event);
    let event_pubkey = &db_event.pubkey;
    let last_update = db_event.remote_creation().ok_or(Error::NotConfirmedEvent)?;
    let metadata = Metadata::from_json(&db_event.content)
        .map_err(|_| Error::JsonToMetadata(db_event.content.to_string()))?;

    let event = if event_pubkey == &keys.public_key() {
        handle_user_metadata_event(pool, relay_url, &metadata, &last_update).await?
    } else {
        handle_contact_metadata_event(pool, relay_url, &metadata, event_pubkey, &last_update)
            .await?
    };

    Ok(event)
}

// Handle user metadata events and update user profile metadata if needed.
async fn handle_user_metadata_event(
    pool: &SqlitePool,
    relay_url: &Url,
    metadata: &Metadata,
    last_update: &NaiveDateTime,
) -> Result<Event, Error> {
    tracing::debug!("handle_user_metadata_event");
    if UserConfig::should_update_user_metadata(pool, last_update).await? {
        UserConfig::update_user_metadata(metadata, last_update, pool).await?;
        Ok(Event::UpdatedUserProfileMeta {
            relay_url: relay_url.clone(),
            metadata: metadata.clone(),
        })
    } else {
        tracing::warn!("Received outdated metadata for user");
        Ok(Event::None)
    }
}

// Handle contact metadata events and update contact metadata if needed.
async fn handle_contact_metadata_event(
    pool: &SqlitePool,
    relay_url: &Url,
    metadata: &Metadata,
    pubkey: &XOnlyPublicKey,
    last_update: &NaiveDateTime,
) -> Result<Event, Error> {
    tracing::debug!("handle_contact_metadata_event");
    if let Some(mut db_contact) = DbContact::fetch_one(pool, pubkey).await? {
        if should_update_contact_metadata(&db_contact, last_update) {
            db_contact = db_contact.with_profile_meta(metadata, *last_update);
            DbContact::update(pool, &db_contact).await?;
            tracing::info!("Updated contact with profile metadata");
            tracing::debug!("{:?}", db_contact);
            Ok(Event::UpdatedContactMetadata {
                db_contact,
                relay_url: relay_url.clone(),
            })
        } else {
            tracing::warn!("Received outdated metadata for contact: {}", pubkey);
            Ok(Event::None)
        }
    } else {
        tracing::warn!("Received metadata for unknown contact: {}", pubkey);
        Ok(Event::None)
    }
}

// Determine if the contact metadata should be updated based on the last update time.
fn should_update_contact_metadata(db_contact: &DbContact, last_update: &NaiveDateTime) -> bool {
    db_contact
        .get_profile_meta_last_update()
        .map(|previous_update| previous_update <= *last_update)
        .unwrap_or(true)
}

pub async fn handle_profile_picture_update(
    keys: &Keys,
    public_key: XOnlyPublicKey,
    pool: &SqlitePool,
    path: PathBuf,
) -> Event {
    tracing::debug!("handle_profile_picture_update");
    if keys.public_key() == public_key {
        // user
        match UserConfig::update_user_profile_picture(pool, &path).await {
            Ok(_) => Event::UserProfilePictureUpdated,
            Err(e) => Event::Error(e.to_string()),
        }
    } else {
        match DbContact::fetch_one(pool, &public_key).await {
            Ok(Some(mut db_contact)) => {
                db_contact = db_contact.with_local_profile_image(&path);
                match DbContact::update(pool, &db_contact).await {
                    Ok(_) => Event::ContactUpdated(db_contact.clone()),
                    Err(e) => Event::Error(e.to_string()),
                }
            }
            Ok(None) => Event::None,
            Err(e) => Event::Error(e.to_string()),
        }
    }
}

pub async fn handle_profile_banner_update(
    keys: &Keys,
    public_key: XOnlyPublicKey,
    pool: &SqlitePool,
    path: PathBuf,
) -> Event {
    tracing::debug!("handle_profile_banner_update");
    if keys.public_key() == public_key {
        // user
        match UserConfig::update_user_banner_picture(pool, &path).await {
            Ok(_) => Event::UserBannerPictureUpdated,
            Err(e) => Event::Error(e.to_string()),
        }
    } else {
        match DbContact::fetch_one(pool, &public_key).await {
            Ok(Some(mut db_contact)) => {
                db_contact = db_contact.with_local_banner_image(&path);
                match DbContact::update(pool, &db_contact).await {
                    Ok(_) => Event::ContactUpdated(db_contact.clone()),
                    Err(e) => Event::Error(e.to_string()),
                }
            }
            Ok(None) => Event::None,
            Err(e) => Event::Error(e.to_string()),
        }
    }
}

pub async fn update_user_metadata(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    profile_meta: nostr_sdk::Metadata,
) -> Event {
    let last_update = Utc::now().naive_utc();
    match UserConfig::update_user_metadata_if_newer(pool, &profile_meta, last_update).await {
        Ok(_) => match build_profile_event(pool, keys, &profile_meta).await {
            Ok(ns_event) => to_backend_channel(
                back_sender,
                BackEndInput::StorePendingMetadata((ns_event, profile_meta)),
            ),
            Err(e) => Event::Error(e.to_string()),
        },
        Err(e) => Event::Error(e.to_string()),
    }
}
