use iced::widget::{button, column};
use nostr_sdk::Metadata;

use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::net::events::Event;
use crate::widget::Element;

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
    BackEndEvent(Event),
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
            Message::BackEndEvent(_ev) => (),
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
            TextInputGroup::new("Name", &self.name, Message::ProfileNameChange)
                .placeholder("Name")
                .build();

        let user_name_input =
            TextInputGroup::new("Username", &self.user_name, Message::UserNameChange)
                .placeholder("Username")
                .build();

        let picture_url_input =
            TextInputGroup::new("Picture Url", &self.picture_url, Message::PictureUrlChange)
                .placeholder("https://my-picture.com/img/bitcoin_is_king.jpg")
                .build();

        let about_input = TextInputGroup::new("About", &self.about, Message::AboutChange)
            .placeholder("About you, what you like, what you post, etc.")
            .build();

        let banner_input = TextInputGroup::new("Banner", &self.banner, Message::BannerChange)
            .placeholder("https://my-picture.com/img/bitcoin_is_king.jpg")
            .build();

        let website_input = TextInputGroup::new("Website", &self.website, Message::WebsiteChange)
            .placeholder("https://my-website-rocks.com")
            .build();

        let ln_input = TextInputGroup::new(
            "Lightning Network Address (LUD 16)",
            &self.ln_addrs,
            Message::LNChange,
        )
        .placeholder("my-ln-address@walletofsatoshi.com")
        .tooltip("Some wallets support Lightning Network Address")
        .build();

        let nostr_addrs_input = TextInputGroup::new(
            "Nostr Address (NIP 05)",
            &self.nostr_addrs,
            Message::NIP05Change,
        )
        .placeholder("my-addrs@example.com")
        .build();

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
