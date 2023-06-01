use std::collections::HashSet;

use crate::{consts::default_channel_image, db::ImageDownloaded, net::ImageSize, Error};
use chrono::NaiveDateTime;
use iced::widget::image::Handle;
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use url::Url;

use crate::utils::ns_event_to_naive;

use super::ChannelMetadata;

// todo: maybe use channel_id?
#[derive(Debug, Clone)]
pub struct ChannelResult {
    pub id: EventId,
    pub url: Url,
    pub name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub members: HashSet<XOnlyPublicKey>,
    pub created_at: NaiveDateTime,
    pub creator: XOnlyPublicKey,
    pub loading_details: bool,
    pub image_handle: Handle,
}

impl ChannelResult {
    pub fn from_ns_event(url: Url, ns_event: nostr::Event) -> Result<ChannelResult, Error> {
        let metadata = ChannelMetadata::from_json(&ns_event.content)?;
        let mut members = HashSet::new();
        let creator = ns_event.pubkey;
        members.insert(creator.clone());

        Ok(ChannelResult {
            id: ns_event.id,
            url,
            name: metadata.name,
            about: metadata.about,
            picture: metadata.picture,
            creator,
            members,
            created_at: ns_event_to_naive(ns_event.created_at)?,
            loading_details: true,
            image_handle: Handle::from_memory(default_channel_image(ImageSize::Medium)),
        })
    }

    pub fn loading_details(&mut self) {
        self.loading_details = true;
    }

    pub fn done_loading(&mut self) {
        self.loading_details = false;
    }

    pub fn message(&mut self, ns_event: nostr::Event) {
        self.members.insert(ns_event.pubkey);
    }

    pub fn update_meta(&mut self, ns_event: nostr::Event) -> Result<(), Error> {
        let metadata = ChannelMetadata::from_json(&ns_event.content)?;
        self.name = metadata.name;
        self.about = metadata.about;
        self.picture = metadata.picture;
        Ok(())
    }

    pub fn update_image(&mut self, image: ImageDownloaded) {
        self.image_handle = Handle::from_path(image.path);
    }
}
