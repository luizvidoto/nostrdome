use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Alignment, Length};
use nostr::Metadata;

use crate::components::common_scrollable;
use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::db::{DbRelay, DbRelayResponse};
use crate::icon::satellite_icon;
use crate::net::BackEndConnection;
use crate::net::{self, events::Event};
use crate::style;
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
    RelaysConfirmationPress(Option<AccountRelaysResponse>),
}

#[derive(Debug, Clone)]
pub struct AccountRelaysResponse {
    pub confirmed_relays: Vec<DbRelayResponse>,
    pub all_relays: Vec<DbRelay>,
}
impl AccountRelaysResponse {
    fn new(
        confirmed_relays: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    ) -> AccountRelaysResponse {
        Self {
            confirmed_relays,
            all_relays,
        }
    }
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
    relays_response: Option<AccountRelaysResponse>,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::GetUserProfileMeta);
        conn.send(net::ToBackend::FetchRelayResponsesUserProfile);
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
            relays_response: None,
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::RelaysConfirmationPress(_) => (),
            Message::BackEndEvent(back_ev) => match back_ev {
                Event::ConfirmedMetadata { is_user, .. } => {
                    if is_user {
                        conn.send(net::ToBackend::FetchRelayResponsesUserProfile);
                    }
                }
                Event::GotRelayResponsesUserProfile {
                    responses,
                    all_relays,
                } => {
                    self.relays_response = Some(AccountRelaysResponse::new(responses, all_relays));
                }
                Event::GotUserProfileCache(cache) => {
                    if let Some(profile_cache) = cache {
                        let meta = profile_cache.metadata;
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
                    conn.send(net::ToBackend::UpdateUserProfileMeta(meta))
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
            match nostr::Url::parse(&self.picture_url) {
                Ok(picture_url) => meta = meta.picture(picture_url),
                Err(_e) => self.picture_url_is_invalid = true,
            }
        }

        if !self.banner.is_empty() {
            match nostr::Url::parse(&self.banner) {
                Ok(banner_url) => meta = meta.banner(banner_url),
                Err(_e) => self.banner_url_is_invalid = true,
            }
        }

        if !self.website.is_empty() {
            match nostr::Url::parse(&self.website) {
                Ok(website_url) => meta = meta.website(website_url),
                Err(_e) => self.website_url_is_invalid = true,
            }
        }
        meta
    }
    fn all_valid(&self) -> bool {
        !self.picture_url_is_invalid && !self.website_url_is_invalid && !self.banner_url_is_invalid
    }
    fn make_relays_response<'a>(&self) -> Element<'a, Message> {
        if let Some(response) = &self.relays_response {
            let resp_txt = format!(
                "{}/{}",
                &response.confirmed_relays.len(),
                &response.all_relays.len()
            );
            tooltip(
                button(
                    row![
                        text(&resp_txt).size(18).style(style::Text::Placeholder),
                        satellite_icon().size(16).style(style::Text::Placeholder)
                    ]
                    .spacing(5),
                )
                .on_press(Message::RelaysConfirmationPress(
                    self.relays_response.to_owned(),
                ))
                .style(style::Button::MenuBtn)
                .padding(5),
                "Relays Confirmation",
                tooltip::Position::Left,
            )
            .style(style::Container::TooltipBg)
            .into()
        } else {
            text("Profile not confirmed on any relays")
                .size(18)
                .style(style::Text::Placeholder)
                .into()
        }
    }
    pub fn view(&self) -> Element<Message> {
        let title = title("Account");
        let title_group = container(
            row![
                title,
                Space::with_width(Length::Fill),
                self.make_relays_response()
            ]
            .align_items(Alignment::Center)
            .spacing(10),
        )
        .width(Length::Fill)
        .height(HEADER_HEIGHT);

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
                .placeholder("LNURL1DP68GURN8GHJ7UM9WFMXJCM99E3K7MF0V9CXJ0M385EKVCENXC6R2C35XVUKXEFCV5MKVV34X5EKZD3EV56NYD3HXQURZEPEXEJXXEPNXSCRVWFNV9NXZCN9XQ6XYEFHVGCXXCMYXYMNSERXFQ5FNS")
                .tooltip("Pay Request URL")
                .build();

        let ln_input = TextInputGroup::new(
            "Lightning Network Address (LUD 16)",
            &self.ln_addrs,
            Message::LNChange,
        )
        .placeholder("my-ln-address@getalby.com")
        .tooltip("Email like address for the Lightning Network")
        .build();

        let nostr_addrs_input = TextInputGroup::new(
            "Nostr Address (NIP 05)",
            &self.nostr_addrs,
            Message::NIP05Change,
        )
        .placeholder("my-addrs@example.com")
        .tooltip("Easily find and confirm users using their email-like identifiers on NOSTR")
        .build();

        let form = container(common_scrollable(
            column![
                profile_name_input,
                user_name_input,
                about_input,
                picture_url_input.build(),
                banner_input.build(),
                website_input.build(),
                ln_url_input,
                ln_input,
                nostr_addrs_input,
            ]
            .spacing(10),
        ))
        .width(Length::Fill)
        .height(Length::Fill);

        let mut save_btn = button("Save").padding(10);
        if self.all_valid() {
            save_btn = save_btn.on_press(Message::SavePress);
        }
        let footer_row = container(row![Space::with_width(Length::Fill), save_btn].spacing(10))
            .width(Length::Fill)
            .height(FOOTER_HEIGHT);

        container(column![title_group, form, footer_row].spacing(10)).into()
    }
}

const HEADER_HEIGHT: f32 = 50.0;
const FOOTER_HEIGHT: f32 = 50.0;
