use std::str::FromStr;

use chrono::Utc;
use iced::widget::{button, column, container, radio, row, scrollable, text, text_input};
use iced::Length;
use iced_style::Color;
use nostr::secp256k1::XOnlyPublicKey;
use nostr::EventId;
use once_cell::sync::Lazy;

use crate::components::chat_contact::ChatContact;
use crate::components::chat_list_container::{self, ChatListContainer};
use crate::components::common_scrollable;
use crate::components::text::title;
use crate::db::{DbContact, MessageStatus};
use crate::net::{BackEndConnection, BackendEvent, ToBackend};
use crate::style::{AppPalette, Theme};
use crate::types::ChatMessage;
use crate::{style, widget::Element};

use super::route::Route;
use super::RouterCommand;

static CHAT_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ChangeTheme(style::Theme),
    ChangeBackground(Color),
    ChangeChatBg(Color),
    ChangePrimary(Color),
    ChangePrimaryOpaque(Color),
    ChangeDanger(Color),
    ChangeTextColor(Color),
    ChangeGrayish(Color),
    ChangeChatSearchInputBg(Color),
    ChangeStatusBarBg(Color),
    ChangeNotification(Color),
    ChangeStatusBarTextColor(Color),
    ChangeHoveredBgScrollbar(Color),
    ChangeHoveredBgScroller(Color),
    ChangeHoveredBgScrollbarMo(Color),
    ChangeHoveredBgScrollerMo(Color),
    ChangeHoverStatusBarBg(Color),
    ChangeWelcomeBg1(Color),
    ChangeWelcomeBg2(Color),
    ChangeWelcomeBg3(Color),
}

pub struct State {
    app_palette: Option<AppPalette>,
    chat_ct: ChatListContainer,
    chat_msgs: Vec<ChatMessage>,
    fake_chat_contact: ChatContact,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(ToBackend::GetTheme);
        let pubkey = XOnlyPublicKey::from_str(
            "8860df7d3b24bfb40fe5bdd2041663d35d3e524ce7376628aa55a7d3e624ac46",
        )
        .unwrap();
        let db_contact = DbContact::new(&pubkey);
        Self {
            app_palette: None,
            chat_ct: ChatListContainer::new(),
            chat_msgs: make_chat_msgs(),
            fake_chat_contact: ChatContact::new(0, &db_contact, conn),
        }
    }
}
impl Route for State {
    type Message = Message;
    fn backend_event(
        &mut self,
        event: crate::net::BackendEvent,
        _conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        let command = RouterCommand::new();
        match event {
            BackendEvent::ThemeChanged(theme) | BackendEvent::GotTheme(theme) => {
                self.app_palette = Some(theme.palette());
            }
            _ => (),
        }
        command
    }
    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> super::RouterCommand<Self::Message> {
        let command = RouterCommand::new();
        match self.app_palette {
            Some(mut palette) => match message {
                Message::None => (),
                Message::ChangeTheme(theme) => conn.send(ToBackend::SetTheme(theme)),

                Message::ChangeBackground(color) => {
                    palette.background = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeChatBg(color) => {
                    palette.chat_bg = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangePrimary(color) => {
                    palette.primary = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangePrimaryOpaque(color) => {
                    palette.primary_opaque = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeDanger(color) => {
                    palette.danger = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeTextColor(color) => {
                    palette.text_color = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeGrayish(color) => {
                    palette.grayish = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeChatSearchInputBg(color) => {
                    palette.chat_search_input_bg = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeStatusBarBg(color) => {
                    palette.status_bar_bg = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeNotification(color) => {
                    palette.notification = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeStatusBarTextColor(color) => {
                    palette.status_bar_text_color = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeHoveredBgScrollbar(color) => {
                    palette.hovered_bg_scrollbar = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeHoveredBgScroller(color) => {
                    palette.hovered_bg_scroller = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeHoveredBgScrollbarMo(color) => {
                    palette.hovered_bg_scrollbar_mo = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeHoveredBgScrollerMo(color) => {
                    palette.hovered_bg_scroller_mo = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeHoverStatusBarBg(color) => {
                    palette.hover_status_bar_bg = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeWelcomeBg1(color) => {
                    palette.welcome_bg_1 = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeWelcomeBg2(color) => {
                    palette.welcome_bg_2 = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
                Message::ChangeWelcomeBg3(color) => {
                    palette.welcome_bg_3 = color;
                    conn.send(ToBackend::SetTheme(Theme::Custom(palette)))
                }
            },
            None => (),
        }

        command
    }
    fn view(&self, selected_theme: Option<Theme>) -> Element<'_, Self::Message> {
        let title = title("Theme Styling");

        if let Some(palette) = self.app_palette {
            let radio_buttons = column![
                radio(
                    "Light",
                    RadioTheme::from(Theme::Light),
                    selected_theme.map(RadioTheme::from),
                    |radio_theme| Message::ChangeTheme(radio_theme.into())
                ),
                radio(
                    "Dark",
                    RadioTheme::from(Theme::Dark),
                    selected_theme.map(RadioTheme::from),
                    |radio_theme| Message::ChangeTheme(radio_theme.into())
                ),
                radio(
                    "Custom",
                    RadioTheme::from(Theme::Custom(AppPalette::default())),
                    selected_theme.map(RadioTheme::from),
                    |radio_theme| Message::ChangeTheme(radio_theme.into())
                ),
            ];

            let background_controls = column![
                text("Background Color").size(24),
                color_control::view(palette.background.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangeBackground(color),
                }),
            ]
            .spacing(5);

            let chat_bg_controls = column![
                text("Chat Bg Color").size(24),
                color_control::view(palette.chat_bg.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangeChatBg(color),
                }),
            ]
            .spacing(5);

            let primary_controls = column![
                text("Primary Color").size(24),
                color_control::view(palette.primary.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangePrimary(color),
                }),
            ]
            .spacing(5);

            let primary_opaque_controls = column![
                text("Primary Opaque Color").size(24),
                color_control::view(palette.primary_opaque.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) =>
                        Message::ChangePrimaryOpaque(color),
                }),
            ]
            .spacing(5);

            let danger_controls = column![
                text("Danger Color").size(24),
                color_control::view(palette.danger.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangeDanger(color),
                }),
            ]
            .spacing(5);

            let text_color_controls = column![
                text("Text Color").size(24),
                color_control::view(palette.text_color.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangeTextColor(color),
                }),
            ]
            .spacing(5);

            let grayish_controls = column![
                text("Grayish Color").size(24),
                color_control::view(palette.grayish.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) => Message::ChangeGrayish(color),
                }),
            ]
            .spacing(5);

            let chat_search_input_controls = column![
                text("Chat Search Input Color").size(24),
                color_control::view(palette.chat_search_input_bg.to_owned()).map(|m| {
                    match m {
                        color_control::Message::ColorChanged(color) => {
                            Message::ChangeChatSearchInputBg(color)
                        }
                    }
                }),
            ]
            .spacing(5);

            let status_bar_bg_controls = column![
                text("Status Bar Bg Color").size(24),
                color_control::view(palette.status_bar_bg.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) =>
                        Message::ChangeStatusBarBg(color),
                }),
            ]
            .spacing(5);

            let status_bar_text_controls = column![
                text("Status Bar Text Color").size(24),
                color_control::view(palette.status_bar_text_color.to_owned()).map(|m| {
                    match m {
                        color_control::Message::ColorChanged(color) => {
                            Message::ChangeStatusBarTextColor(color)
                        }
                    }
                }),
            ]
            .spacing(5);

            let hover_status_bar_bg_controls = column![
                text("Hover Status Bar Bg Color").size(24),
                color_control::view(palette.hover_status_bar_bg.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) =>
                        Message::ChangeHoverStatusBarBg(color),
                }),
            ]
            .spacing(5);

            let hovered_bg_scrollbar_controls = column![
                text("Hovered Bg Scrollbar Color").size(24),
                color_control::view(palette.hovered_bg_scrollbar.to_owned()).map(|m| {
                    match m {
                        color_control::Message::ColorChanged(color) => {
                            Message::ChangeHoveredBgScrollbar(color)
                        }
                    }
                }),
            ]
            .spacing(5);

            let hovered_bg_scrollbar_mo_controls = column![
                text("Hovered Bg Scrollbar Mo Color").size(24),
                color_control::view(palette.hovered_bg_scrollbar_mo.to_owned()).map(|m| {
                    match m {
                        color_control::Message::ColorChanged(color) => {
                            Message::ChangeHoveredBgScrollbarMo(color)
                        }
                    }
                }),
            ]
            .spacing(5);

            let hovered_bg_scroller_controls = column![
                text("Hovered Bg Scroller Color").size(24),
                color_control::view(palette.hovered_bg_scroller.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) =>
                        Message::ChangeHoveredBgScroller(color),
                }),
            ]
            .spacing(5);

            let hovered_bg_scroller_mo_controls = column![
                text("Hovered Bg Scroller Mo Color").size(24),
                color_control::view(palette.hovered_bg_scroller_mo.to_owned()).map(|m| {
                    match m {
                        color_control::Message::ColorChanged(color) => {
                            Message::ChangeHoveredBgScrollerMo(color)
                        }
                    }
                }),
            ]
            .spacing(5);

            let notification_controls = column![
                text("Notification Color").size(24),
                color_control::view(palette.notification.to_owned()).map(|m| match m {
                    color_control::Message::ColorChanged(color) =>
                        Message::ChangeNotification(color),
                }),
            ]
            .spacing(5);

            let color_controls = column![
                background_controls,
                chat_bg_controls,
                primary_controls,
                primary_opaque_controls,
                danger_controls,
                text_color_controls,
                notification_controls,
                grayish_controls,
                chat_search_input_controls,
                status_bar_bg_controls,
                status_bar_text_controls,
                hover_status_bar_bg_controls,
                hovered_bg_scrollbar_controls,
                hovered_bg_scrollbar_mo_controls,
                hovered_bg_scroller_controls,
                hovered_bg_scroller_mo_controls,
            ]
            .spacing(10);

            let chat_container = container(
                self.chat_ct
                    .view(
                        &CHAT_SCROLLABLE_ID,
                        &CHAT_INPUT_ID,
                        &self.chat_msgs,
                        Some(&self.fake_chat_contact),
                    )
                    .map(|m| match m {
                        chat_list_container::Message::DMSentPress(_) => Message::None,
                        chat_list_container::Message::DMNMessageChange(_) => Message::None,
                        chat_list_container::Message::GotChatSize(_, _) => Message::None,
                        chat_list_container::Message::Scrolled(_) => Message::None,
                        chat_list_container::Message::OpenContactProfile => Message::None,
                        chat_list_container::Message::ChatRightClick(_, _) => Message::None,
                    }),
            )
            .padding(5)
            .style(style::Container::Bordered)
            .height(400)
            .width(400);

            let left = container(
                common_scrollable(
                    column![all_buttons(), chat_container]
                        .spacing(20)
                        .padding([20, 0, 0, 20]),
                )
                .width(Length::Fill),
            )
            .width(Length::Fill);

            let right = container(common_scrollable(
                column![title, radio_buttons, color_controls]
                    .spacing(20)
                    .padding([20, 0, 0, 20]),
            ))
            .width(CONTROLS_WIDTH + 70);

            row![left, right].into()
        } else {
            text("Loading palette...").into()
        }
    }
}

const CONTROLS_WIDTH: u16 = 200;

fn all_buttons() -> Element<'static, Message> {
    let primary_btn = button("Primary")
        .style(style::Button::Primary)
        .on_press(Message::None);
    let danger_btn = button("Danger")
        .style(style::Button::Danger)
        .on_press(Message::None);
    let invisible_btn = button("Invisible")
        .style(style::Button::Invisible)
        .on_press(Message::None);
    let contact_card_btn = button("ContactCard")
        .style(style::Button::ContactCard)
        .on_press(Message::None);
    let active_contact_card_btn = button("ActiveContactCard")
        .style(style::Button::ActiveContactCard)
        .on_press(Message::None);
    let menu_btn = button("Menu")
        .style(style::Button::MenuBtn)
        .on_press(Message::None);
    let active_menu_btn = button("ActiveMenu")
        .style(style::Button::ActiveMenuBtn)
        .on_press(Message::None);
    let status_bar_btn = button("StatusBar")
        .style(style::Button::StatusBarButton)
        .on_press(Message::None);
    let bordered_btn = button("Bordered")
        .style(style::Button::Bordered)
        .on_press(Message::None);
    let notification_btn = button("10")
        .style(style::Button::Notification)
        .on_press(Message::None);
    let context_menu_btn = button("ContextMenuButton")
        .style(style::Button::ContextMenuButton)
        .on_press(Message::None);
    let link_btn = button("Link")
        .style(style::Button::Link)
        .on_press(Message::None);

    column![
        primary_btn,
        danger_btn,
        invisible_btn,
        contact_card_btn,
        active_contact_card_btn,
        menu_btn,
        active_menu_btn,
        status_bar_btn,
        bordered_btn,
        notification_btn,
        context_menu_btn,
        link_btn,
    ]
    .spacing(5)
    .into()
}

mod color_control {
    use crate::widget::Element;
    use iced::widget::{column, row, slider, text};
    use iced_style::Color;

    use super::CONTROLS_WIDTH;

    #[derive(Debug, Clone)]
    pub enum Message {
        ColorChanged(Color),
    }

    pub fn view(color: Color) -> Element<'static, Message> {
        let red_f32 = color.r * 255.0;
        let green_f32 = color.g * 255.0;
        let blue_f32 = color.b * 255.0;
        let range = 0.0..=255.0;

        let r_control = slider(range.clone(), red_f32, move |value| {
            on_red_change(color, value)
        })
        .width(CONTROLS_WIDTH);

        let g_control = slider(range.clone(), green_f32, move |value| {
            on_green_change(color, value)
        })
        .width(CONTROLS_WIDTH);

        let b_control =
            slider(range, blue_f32, move |value| on_blue_change(color, value)).width(200);

        let a_control = slider(0.0..=1.0, color.a, move |value| {
            on_alpha_change(color, value)
        })
        .step(0.05)
        .width(CONTROLS_WIDTH);

        column![
            row![r_control, text(red_f32.round().to_string())].spacing(5),
            row![g_control, text(green_f32.round().to_string())].spacing(5),
            row![b_control, text(blue_f32.round().to_string())].spacing(5),
            row![a_control, text(color.a.to_string())].spacing(5),
        ]
        .spacing(10)
        .into()
    }

    fn on_alpha_change(mut color: Color, value: f32) -> Message {
        color.a = value;
        Message::ColorChanged(color)
    }

    fn on_red_change(mut color: Color, value: f32) -> Message {
        color.r = value / 255.0;
        Message::ColorChanged(color)
    }
    fn on_green_change(mut color: Color, value: f32) -> Message {
        color.g = value / 255.0;
        Message::ColorChanged(color)
    }
    fn on_blue_change(mut color: Color, value: f32) -> Message {
        color.b = value / 255.0;
        Message::ColorChanged(color)
    }
}

fn make_chat_msgs() -> Vec<ChatMessage> {
    vec![
        (false, "eai cara"),
        (true, "e ai de boas??"),
        (false, "to bem e vc?"),
        (true, "to bemzasso e ai?"),
        (false, "de boas"),
        (false, "bora pro role?"),
        (false, "não vai??"),
        (true, "bora pourra"),
        (true, "vamos sim"),
    ]
    .into_iter()
    .enumerate()
    .map(|(idx, (is_from_user, msg))| ChatMessage {
        msg_id: idx as i64,
        display_time: Utc::now().naive_utc(),
        content: msg.to_owned(),
        is_from_user,
        select_name: "Jonas".into(),
        event_id: None,
        event_hash: EventId::from_hex(
            "8233a5d8e27a9415d22c974d70935011664ada55ae3152bd10d697d3a3c74f67",
        )
        .unwrap(),
        status: MessageStatus::Seen,
    })
    .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct RadioTheme(u8);

impl PartialEq for RadioTheme {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for RadioTheme {}

impl From<Theme> for RadioTheme {
    fn from(theme: Theme) -> Self {
        let theme_num: u8 = theme.into();
        Self(theme_num)
    }
}

impl Into<Theme> for RadioTheme {
    fn into(self) -> Theme {
        match self.0 {
            0 => Theme::Light,
            1 => Theme::Dark,
            2 => Theme::Custom(AppPalette::default()), // Just an example, replace this with actual custom palette
            _ => unreachable!(),                       // Can only be 0, 1, or 2
        }
    }
}
