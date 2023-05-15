use std::path::PathBuf;

use chrono::NaiveDateTime;
use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Metadata};
use sqlx::SqlitePool;

use crate::net::{events::backend::BackEndInput, reqwest_client::download_image, ImageKind};

pub async fn download_profile_image(
    cache_pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    event_date: &NaiveDateTime,
    public_key: &XOnlyPublicKey,
) {
    download(
        cache_pool,
        back_sender,
        metadata,
        event_date,
        public_key,
        ImageKind::Profile,
        "Downloaded profile picture for pubkey: ",
        |(public_key, path)| BackEndInput::ProfilePictureDownloaded((public_key, path)),
    )
    .await;
}

pub async fn _download_banner_image(
    cache_pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    event_date: &NaiveDateTime,
    public_key: &XOnlyPublicKey,
) {
    download(
        cache_pool,
        back_sender,
        metadata,
        event_date,
        public_key,
        ImageKind::Banner,
        "Downloaded banner picture for pubkey: ",
        |(public_key, path)| BackEndInput::ProfileBannerDownloaded((public_key, path)),
    )
    .await;
}

async fn download(
    cache_pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    event_date: &NaiveDateTime,
    public_key: &XOnlyPublicKey,
    kind: ImageKind,
    info_msg: &str,
    back_input: impl FnOnce((XOnlyPublicKey, PathBuf)) -> BackEndInput + Send + 'static,
) {
    if let Some(picture_url) = &metadata.picture {
        let mut sender_1 = back_sender.clone();
        let pic_1 = picture_url.clone();
        let pub_1 = public_key.clone();
        let cache_pool_1 = cache_pool.clone();
        let event_date_1 = event_date.clone();
        let info_msg_1 = info_msg.to_string();
        tokio::spawn(async move {
            match download_image(&cache_pool_1, &pic_1, &event_date_1, &pub_1, kind).await {
                Ok(path) => {
                    tracing::info!("{}", info_msg_1);
                    tracing::info!("{:?}", &path);
                    let message = back_input((pub_1, path.clone()));
                    if let Err(e) = sender_1.try_send(message) {
                        tracing::error!("Error sending message to backend: {:?}", e);
                    }
                }
                Err(e) => {
                    if let Err(e) = sender_1.try_send(BackEndInput::Error(e.to_string())) {
                        tracing::error!("Error sending error to backend: {:?}", e);
                    }
                }
            }
        });
    } else {
        tracing::warn!("No profile picture to download");
    }
}
