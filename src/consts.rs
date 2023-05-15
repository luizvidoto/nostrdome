use crate::net::ImageSize;

pub(crate) const APP_PROJECT_DIRS: (&'static str, &'static str, &'static str) =
    ("", "", "NostrTalk");
pub(crate) const WELCOME_IMAGE: &[u8] = include_bytes!("../assets/welcome_img.jpg");
pub(crate) const RELAYS_IMAGE: &[u8] = include_bytes!("../assets/relays_img_2.jpg");
pub(crate) const CONTACTS_IMAGE: &[u8] = include_bytes!("../assets/contacts_img_3.png");
pub(crate) const DEFAULT_PROFILE_IMAGE_SMALL: &[u8] =
    include_bytes!("../assets/default_profile_image_small.png");
pub(crate) const DEFAULT_PROFILE_IMAGE_MEDIUM: &[u8] =
    include_bytes!("../assets/default_profile_image_medium.png");
pub(crate) const DEFAULT_PROFILE_IMAGE_ORIGINAL: &[u8] =
    include_bytes!("../assets/default_profile_image_original.png");

pub(crate) const SMALL_PROFILE_PIC_WIDTH: u32 = 50;
pub(crate) const SMALL_PROFILE_PIC_HEIGHT: u32 = 50;
pub(crate) const MEDIUM_PROFILE_PIC_WIDTH: u32 = 200;
pub(crate) const MEDIUM_PROFILE_PIC_HEIGHT: u32 = 200;

pub(crate) const YMD_FORMAT: &'static str = "%Y-%m-%d";

pub const fn default_profile_image(size: ImageSize) -> &'static [u8] {
    match size {
        ImageSize::Small => DEFAULT_PROFILE_IMAGE_SMALL,
        ImageSize::Medium => DEFAULT_PROFILE_IMAGE_MEDIUM,
        ImageSize::Original => DEFAULT_PROFILE_IMAGE_MEDIUM,
    }
}
