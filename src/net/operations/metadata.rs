use futures::channel::mpsc;
use nostr::{Keys, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbEvent, ProfileCache},
    error::Error,
    net::{
        events::{backend::BackEndInput, Event},
        to_backend_channel,
    },
};

use super::builder::build_profile_event;

// Handle metadata events and update user profile or contact metadata accordingly.
pub async fn handle_metadata_event(
    cache_pool: &SqlitePool,
    db_event: DbEvent,
) -> Result<Event, Error> {
    tracing::debug!("handle_metadata_event");
    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::debug!("{:?}", db_event);

    let rows_changed = ProfileCache::insert_with_event(cache_pool, &db_event).await?;

    if rows_changed == 0 {
        tracing::info!("Cache already up to date");
        return Ok(Event::None);
    }

    Ok(Event::UpdatedMetadata(db_event.pubkey))
}

// Handle user metadata events and update user profile metadata if needed.
// async fn handle_user_metadata_event(
//     pool: &SqlitePool,
//     relay_url: &Url,
//     metadata: &Metadata,
//     last_update: &NaiveDateTime,
// ) -> Result<Event, Error> {
//     tracing::debug!("handle_user_metadata_event");
//     if UserConfig::should_update_user_metadata(pool, last_update).await? {
//         UserConfig::update_user_metadata(metadata, last_update, pool).await?;
//         Ok(Event::UpdatedUserProfileMeta {
//             relay_url: relay_url.clone(),
//             metadata: metadata.clone(),
//         })
//     } else {
//         tracing::warn!("Received outdated metadata for user");
//         Ok(Event::None)
//     }
// }

// pub async fn handle_profile_picture_update(
//     keys: &Keys,
//     public_key: XOnlyPublicKey,
//     pool: &SqlitePool,
//     path: PathBuf,
// ) -> Event {
//     tracing::debug!("handle_profile_picture_update");
//     if keys.public_key() == public_key {
//         // user
//         match UserConfig::update_user_profile_picture(pool, &path).await {
//             Ok(_) => Event::UserProfilePictureUpdated,
//             Err(e) => Event::Error(e.to_string()),
//         }
//     } else {
//         match DbContact::fetch_one(pool, &public_key).await {
//             Ok(Some(mut db_contact)) => {
//                 db_contact = db_contact.with_local_profile_image(&path);
//                 match DbContact::update(pool, &db_contact).await {
//                     Ok(_) => Event::ContactUpdated(db_contact.clone()),
//                     Err(e) => Event::Error(e.to_string()),
//                 }
//             }
//             Ok(None) => Event::None,
//             Err(e) => Event::Error(e.to_string()),
//         }
//     }
// }

// pub async fn handle_profile_banner_update(
//     keys: &Keys,
//     public_key: XOnlyPublicKey,
//     pool: &SqlitePool,
//     path: PathBuf,
// ) -> Event {
//     tracing::debug!("handle_profile_banner_update");
//     if keys.public_key() == public_key {
//         // user
//         match UserConfig::update_user_banner_picture(pool, &path).await {
//             Ok(_) => Event::UserBannerPictureUpdated,
//             Err(e) => Event::Error(e.to_string()),
//         }
//     } else {
//         match DbContact::fetch_one(pool, &public_key).await {
//             Ok(Some(mut db_contact)) => {
//                 db_contact = db_contact.with_local_banner_image(&path);
//                 match DbContact::update(pool, &db_contact).await {
//                     Ok(_) => Event::ContactUpdated(db_contact.clone()),
//                     Err(e) => Event::Error(e.to_string()),
//                 }
//             }
//             Ok(None) => Event::None,
//             Err(e) => Event::Error(e.to_string()),
//         }
//     }
// }

pub async fn update_user_metadata(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    profile_meta: nostr::Metadata,
) -> Event {
    match build_profile_event(pool, keys, &profile_meta).await {
        Ok(ns_event) => to_backend_channel(
            back_sender,
            BackEndInput::StorePendingMetadata((ns_event, profile_meta)),
        ),
        Err(e) => Event::Error(e.to_string()),
    }
}
