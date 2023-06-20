use crate::{
    consts::default_channel_image,
    db::{ChannelCache, ImageDownloaded},
    error::BackendClosed,
    net::{BackEndConnection, ImageKind, ImageSize, ToBackend},
};
use chrono::NaiveDateTime;
use iced::widget::image::Handle;
use url::Url;

// todo: maybe use channel_id?
#[derive(Debug, Clone)]
pub struct ChannelResult {
    pub relay_url: Url,
    pub cache: ChannelCache,
    pub loading_details: bool,
    pub image_handle: Handle,
}

impl ChannelResult {
    pub fn from_cache(url: Url, cache: ChannelCache) -> Self {
        let image_handle = cache
            .image_cache
            .as_ref()
            .map(|image| {
                let path = image.sized_image(IMAGE_SIZE);
                Handle::from_path(path)
            })
            .unwrap_or(Handle::from_memory(default_channel_image(IMAGE_SIZE)));

        Self {
            relay_url: url,
            cache,
            loading_details: true,
            image_handle,
        }
    }
    pub fn done_loading(&mut self) {
        self.loading_details = false;
    }
    pub fn update_cache(
        &mut self,
        new_cache: ChannelCache,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        if let Some(image) = &new_cache.image_cache {
            self.update_image(image);
        } else if let Some(image_url) = &new_cache.metadata.picture {
            conn.send(ToBackend::DownloadImage {
                image_url: image_url.to_owned(),
                kind: ImageKind::Channel,
                identifier: self.cache.channel_id.to_string(),
                event_hash: new_cache.last_event_hash().to_owned(),
            })?;
        } else {
            tracing::info!("Channel don't have image");
        }
        self.cache = new_cache;
        Ok(())
    }
    pub fn update_image(&mut self, image: &ImageDownloaded) {
        let path = image.sized_image(IMAGE_SIZE);
        self.image_handle = Handle::from_path(path);
    }
    pub fn name(&self) -> String {
        self.cache
            .metadata
            .name
            .clone()
            .unwrap_or("Nameless".into())
    }
    pub fn about(&self) -> String {
        self.cache.metadata.about.clone().unwrap_or("".into())
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.cache.created_at
    }
}

const IMAGE_SIZE: ImageSize = ImageSize::Medium;
