use iced::widget::{button, column};
use iced::Element;
use nostr_sdk::Metadata;

use crate::components::text::title;
use crate::components::text_input_group::text_input_group;

#[derive(Debug, Clone)]
pub enum Message {
    ProfileNameChange(String),
    UserNameChange(String),
    PictureUrlChange(String),
    AboutChange(String),
    BannerChange(String),
    WebsiteChange(String),
    LNChange(String),
    NIP05Change(String),
    SubmitPress,
}

#[derive(Debug, Clone)]
pub struct State {
    // profile: Metadata,
    name: String,
    user_name: String,
    picture_url: String,
    about: String,
    banner: String,
    website: String,
    ln_addrs: String,
    nostr_addrs: String,
}
impl State {
    pub fn new(profile: Metadata) -> Self {
        Self {
            // profile: profile.clone(),
            name: profile.name.unwrap_or("".into()),
            user_name: profile.display_name.unwrap_or("".into()),
            picture_url: profile.picture.unwrap_or("".into()),
            about: profile.about.unwrap_or("".into()),
            banner: profile.banner.unwrap_or("".into()),
            website: profile.website.unwrap_or("".into()),
            ln_addrs: profile.lud16.unwrap_or("".into()),
            nostr_addrs: profile.nip05.unwrap_or("".into()),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ProfileNameChange(name) => self.name = name,
            Message::UserNameChange(user_name) => self.user_name = user_name,
            Message::PictureUrlChange(pic_url) => self.picture_url = pic_url,
            Message::AboutChange(about) => self.about = about,
            Message::BannerChange(banner) => self.banner = banner,
            Message::WebsiteChange(website) => self.website = website,
            Message::LNChange(ln_addrs) => self.ln_addrs = ln_addrs,
            Message::NIP05Change(nostr_addrs) => self.nostr_addrs = nostr_addrs,
            Message::SubmitPress => {
                println!("Submit profile data.");
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Account");

        let profile_name_input =
            text_input_group("Name", "Name", &self.name, None, Message::ProfileNameChange);
        let user_name_input = text_input_group(
            "Username",
            "Username",
            &self.user_name,
            None,
            Message::UserNameChange,
        );
        let picture_url_input = text_input_group(
            "Picture Url",
            "https://my-picture.com/img/bitcoin_is_king.jpg",
            &self.picture_url,
            None,
            Message::PictureUrlChange,
        );
        let about_input = text_input_group(
            "About",
            "About you, what you like, what you post, etc.",
            &self.about,
            None,
            Message::AboutChange,
        );
        let banner_input = text_input_group(
            "Banner",
            "https://my-picture.com/img/bitcoin_is_king.jpg",
            &self.banner,
            None,
            Message::BannerChange,
        );
        let website_input = text_input_group(
            "Website",
            "https://my-website-rocks.com",
            &self.website,
            None,
            Message::WebsiteChange,
        );
        let ln_input = text_input_group(
            "Lightning Network Address (LUD 16)",
            "my-ln-address@walletofsatoshi.com",
            &self.ln_addrs,
            Some("Some wallets support Lightning Network Address".to_string()),
            Message::LNChange,
        );
        let nostr_addrs_input = text_input_group(
            "Nostr Address (NIP 05)",
            "my-addrs@example.com",
            &self.nostr_addrs,
            None,
            Message::NIP05Change,
        );

        let submit_btn = button("Submit").on_press(Message::SubmitPress);

        column![
            title,
            profile_name_input,
            user_name_input,
            picture_url_input,
            about_input,
            banner_input,
            website_input,
            ln_input,
            nostr_addrs_input,
            submit_btn
        ]
        .spacing(4)
        .into()
    }
}
