use iced::widget::{button, column, container, row, scrollable, Space};
use iced::Length;
use nostr_sdk::Metadata;

use crate::components::common_scrollable;
use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::net::BackEndConnection;
use crate::net::{self, events::Event};
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    ProfileNameChange(String),
    UserNameChange(String),
    PictureUrlChange(String),
    AboutChange(String),
    BannerChange(String),
    WebsiteChange(String),
    LNURLChange(String),
    LNChange(String),
    NIP05Change(String),
    SavePress,
    BackEndEvent(Event),
}

#[derive(Debug, Clone)]
pub struct State {
    name: String,
    user_name: String,
    picture_url: String,
    about: String,
    banner: String,
    website: String,
    ln_url: String,      //lud06
    ln_addrs: String,    //lud16
    nostr_addrs: String, //nip05
    picture_url_is_invalid: bool,
    website_url_is_invalid: bool,
    banner_url_is_invalid: bool,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::GetUserProfileMeta);
        Self {
            name: "".into(),
            user_name: "".into(),
            picture_url: "".into(),
            about: "".into(),
            banner: "".into(),
            website: "".into(),
            ln_url: "".into(),
            ln_addrs: "".into(),
            nostr_addrs: "".into(),
            picture_url_is_invalid: false,
            website_url_is_invalid: false,
            banner_url_is_invalid: false,
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::BackEndEvent(back_ev) => match back_ev {
                Event::GotUserProfileMeta(meta) => {
                    self.name = meta.name.unwrap_or("".into());
                    self.user_name = meta.display_name.unwrap_or("".into());
                    self.picture_url = meta.picture.unwrap_or("".into());
                    self.about = meta.about.unwrap_or("".into());
                    self.banner = meta.banner.unwrap_or("".into());
                    self.website = meta.website.unwrap_or("".into());
                    self.ln_url = meta.lud06.unwrap_or("".into());
                    self.ln_addrs = meta.lud16.unwrap_or("".into());
                    self.nostr_addrs = meta.nip05.unwrap_or("".into());
                }
                _ => (),
            },
            Message::ProfileNameChange(name) => self.name = name,
            Message::UserNameChange(user_name) => self.user_name = user_name,
            Message::AboutChange(about) => self.about = about,
            Message::PictureUrlChange(pic_url) => {
                self.picture_url = pic_url;
                self.picture_url_is_invalid = false;
            }
            Message::BannerChange(banner) => {
                self.banner = banner;
                self.banner_url_is_invalid = false;
            }
            Message::WebsiteChange(website) => {
                self.website = website;
                self.website_url_is_invalid = false;
            }
            Message::LNURLChange(ln_url) => self.ln_url = ln_url,
            Message::LNChange(ln_addrs) => self.ln_addrs = ln_addrs,
            Message::NIP05Change(nostr_addrs) => self.nostr_addrs = nostr_addrs,
            Message::SavePress => {
                let meta = self.make_meta();
                if self.all_valid() {
                    conn.send(net::Message::UpdateUserProfileMeta(meta))
                }
            }
        }
    }

    fn make_meta(&mut self) -> Metadata {
        let mut meta = Metadata::new()
            .name(self.name.clone())
            .display_name(self.user_name.clone())
            .about(self.about.clone())
            .lud06(self.ln_url.clone())
            .lud16(self.ln_addrs.clone())
            .nip05(self.nostr_addrs.clone());

        if !self.picture_url.is_empty() {
            match nostr_sdk::Url::parse(&self.picture_url) {
                Ok(picture_url) => meta = meta.picture(picture_url),
                Err(_e) => self.picture_url_is_invalid = true,
            }
        }

        if !self.banner.is_empty() {
            match nostr_sdk::Url::parse(&self.banner) {
                Ok(banner_url) => meta = meta.banner(banner_url),
                Err(_e) => self.banner_url_is_invalid = true,
            }
        }

        if !self.website.is_empty() {
            match nostr_sdk::Url::parse(&self.website) {
                Ok(website_url) => meta = meta.website(website_url),
                Err(_e) => self.website_url_is_invalid = true,
            }
        }
        meta
    }
    fn all_valid(&self) -> bool {
        !self.picture_url_is_invalid && !self.website_url_is_invalid && !self.banner_url_is_invalid
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

        let about_input = TextInputGroup::new("About", &self.about, Message::AboutChange)
            .placeholder("About you, what you like, what you post, etc.")
            .build();

        let mut picture_url_input =
            TextInputGroup::new("Picture Url", &self.picture_url, Message::PictureUrlChange)
                .placeholder("https://my-picture.com/img/bitcoin_is_king.jpg");
        if self.picture_url_is_invalid {
            picture_url_input = picture_url_input.invalid("Picture URL is invalid");
        }

        let mut banner_input = TextInputGroup::new("Banner", &self.banner, Message::BannerChange)
            .placeholder("https://my-picture.com/img/bitcoin_is_king.jpg");
        if self.banner_url_is_invalid {
            banner_input = banner_input.invalid("Banner URL is invalid");
        }

        let mut website_input =
            TextInputGroup::new("Website", &self.website, Message::WebsiteChange)
                .placeholder("https://my-website-rocks.com");
        if self.website_url_is_invalid {
            website_input = website_input.invalid("Website URL is invalid");
        }

        let ln_url_input =
            TextInputGroup::new("Lightning URL (LUD 06)", &self.ln_url, Message::LNURLChange)
                .placeholder("my-ln-address@walletofsatoshi.com")
                .tooltip("Some wallets support Lightning Network Address")
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

        let mut save_btn = button("Save");

        if self.all_valid() {
            save_btn = save_btn.on_press(Message::SavePress);
        }

        container(common_scrollable(
            column![
                title,
                profile_name_input,
                user_name_input,
                about_input,
                picture_url_input.build(),
                banner_input.build(),
                website_input.build(),
                ln_url_input,
                ln_input,
                nostr_addrs_input,
                row![Space::with_width(Length::Fill), save_btn].spacing(10)
            ]
            .spacing(4),
        ))
        .into()
    }
}
