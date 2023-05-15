use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Metadata};

use crate::net::{events::backend::BackEndInput, reqwest_client::download_image, ImageKind};

pub async fn download_profile_image(
    back_sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    public_key: &XOnlyPublicKey,
) {
    if let Some(picture_url) = &metadata.picture {
        let mut sender_1 = back_sender.clone();
        let pic_1 = picture_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&pic_1, &pub_1, ImageKind::Profile).await {
                Ok(path) => {
                    tracing::info!("Downloaded profile picture for pubkey: {:?}", &path);
                    if let Err(e) =
                        sender_1.try_send(BackEndInput::ProfilePictureDownloaded((pub_1, path)))
                    {
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

async fn _download_banner_image(
    sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    public_key: &XOnlyPublicKey,
) {
    if let Some(banner_url) = &metadata.banner {
        let mut sender_1 = sender.clone();
        let banner_1 = banner_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&banner_1, &pub_1, ImageKind::Banner).await {
                Ok(path) => {
                    tracing::info!("Downloaded banner picture for pubkey: {:?}", &path);
                    if let Err(e) =
                        sender_1.try_send(BackEndInput::ProfileBannerDownloaded((pub_1, path)))
                    {
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
        tracing::warn!("No banner image to download");
    }
}
