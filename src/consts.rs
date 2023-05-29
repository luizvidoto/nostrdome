use crate::net::ImageSize;

pub(crate) const APP_PROJECT_DIRS: (&'static str, &'static str, &'static str) =
    ("com.nostrtalk", "", "NostrTalk");
pub(crate) const WELCOME_IMAGE: &[u8] = include_bytes!("../assets/welcome_img.jpg");
pub(crate) const RELAYS_IMAGE: &[u8] = include_bytes!("../assets/relays_img_2.jpg");
// pub(crate) const CONTACTS_IMAGE: &[u8] = include_bytes!("../assets/contacts_img_3.png");
pub(crate) const DEFAULT_PROFILE_IMAGE_SMALL: &[u8] =
    include_bytes!("../assets/default_profile_image_small.png");
pub(crate) const DEFAULT_PROFILE_IMAGE_MEDIUM: &[u8] =
    include_bytes!("../assets/default_profile_image_medium.png");
// pub(crate) const DEFAULT_PROFILE_IMAGE_ORIGINAL: &[u8] =
//     include_bytes!("../assets/default_profile_image_original.png");

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

pub(crate) const NOSTRTALK_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const GITHUB_REPO: &'static str = "https://github.com/luizvidoto/nostrtalk";
pub(crate) const BITCOIN_ADDRESS: &'static str = "bc1qr7pavwv0m05gz7x92keuwyqmr6a2yg4jd8vwze";
pub(crate) const LIGHTNING_ADDRESS: &'static str =
    "LNURL1DP68GURN8GHJ7EM9W3SKCCNE9E3K7MF0D3H82UNVWQHKU6TRDD5XUARKHTW6ZP";
pub(crate) const TT_LINK: &'static str = "https://twitter.com/nickhntv";
pub(crate) const NOSTR_RESOURCES_LINK: &'static str = "https://nostr-resources.com";
pub(crate) const RELAY_SUGGESTIONS: [&'static str; 18] = [
    "ws://192.168.15.151:8080",
    "ws://192.168.15.119:8080",
    "wss://relay.plebstr.com",
    "wss://nostr.wine",
    "wss://relay.snort.social",
    "wss://nostr-pub.wellorder.net",
    "wss://relay.damus.io",
    "wss://nostr1.tunnelsats.com",
    "wss://relay.nostr.info",
    "wss://nostr-relay.wlvs.space",
    "wss://nostr.zebedee.cloud",
    "wss://lbrygen.xyz",
    "wss://nostr.8e23.net",
    "wss://nostr.xmr.rocks",
    "wss://xmr.usenostr.org",
    "wss://relay.roli.social",
    "wss://relay.nostr.ro",
    "wss://nostr.swiss-enigma.ch",
];
