use std::path::PathBuf;

use nostr::{secp256k1::XOnlyPublicKey, Keys};
use sqlx::SqlitePool;

use crate::Error;
use crate::{
    db::{profile_cache::ProfileCache, DbContact},
    net::{BackendEvent, ImageKind},
};

pub async fn update_profile_cache(
    pool: &SqlitePool,
    cache_pool: &SqlitePool,
    keys: &Keys,
    public_key: XOnlyPublicKey,
    kind: ImageKind,
    path: PathBuf,
) -> Result<BackendEvent, Error> {
    let _ = ProfileCache::update_local_path(cache_pool, &public_key, kind, &path).await?;

    let event = match keys.public_key() == public_key {
        true => match kind {
            ImageKind::Profile => BackendEvent::UserProfilePictureUpdated(path),
            ImageKind::Banner => BackendEvent::UserBannerPictureUpdated(path),
        },
        false => {
            if let Some(db_contact) = DbContact::fetch_one(pool, cache_pool, &public_key).await? {
                BackendEvent::ContactUpdated(db_contact)
            } else {
                tracing::warn!(
                    "Image downloaded for contact not found in database: {}",
                    public_key
                );
                return Err(Error::ContactNotFound(public_key.clone()));
            }
        }
    };

    Ok(event)
}
