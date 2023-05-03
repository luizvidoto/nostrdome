use iced::widget::{button, checkbox, container, radio, scrollable, text, text_input};
use iced::{application, Color, Vector};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}
impl Theme {
    pub fn pallete(&self) -> AppPalette {
        match self {
            Theme::Light => AppPalette::LIGHT,
            Theme::Dark => AppPalette::DARK,
        }
    }
}
// telegram active
// Color::from_rgb8(65, 159, 217)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppPalette {
    pub background: Color,
    pub chat_bg: Color,
    pub send_button: Color,
    pub text_color: Color,
    pub link_1: Color,
    pub link_2: Color,
    pub icons: Color,
    pub placeholder: Color,
    pub text_selection: Color,
    pub contact_selected: Color,
    pub contact_hover: Color,
    pub contact: Color,
    pub hovered_bg_scrollbar: Color,
    pub hovered_bg_scroller: Color,
    pub hovered_bg_scrollbar_mo: Color,
    pub hovered_bg_scroller_mo: Color,
    pub sent_message_bg: Color,
    pub sent_message_status: Color,
    pub sent_message_date: Color,
    pub received_message_bg: Color,
    pub chat_divider_bg: Color,
    pub chat_search_input_bg: Color,
    pub status_bar_bg: Color,
    pub hover_status_bar_bg: Color,
}
impl AppPalette {
    pub const LIGHT: Self = Self {
        background: Color::from_rgb(23.0 / 255.0, 33.0 / 255.0, 43.0 / 255.0),
        chat_bg: Color::from_rgb(14.0 / 255.0, 22.0 / 255.0, 33.0 / 255.0),
        send_button: Color::from_rgb(82.0 / 255.0, 136.0 / 255.0, 193.0 / 255.0),
        text_color: Color::from_rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),
        link_1: Color::from_rgb(106.0 / 255.0, 179.0 / 255.0, 243.0 / 255.0),
        link_2: Color::from_rgb(115.0 / 255.0, 185.0 / 255.0, 245.0 / 255.0),
        icons: Color::from_rgb(108.0 / 255.0, 120.0 / 255.0, 131.0 / 255.0),
        placeholder: Color::from_rgb(127.0 / 255.0, 145.0 / 255.0, 164.0 / 255.0),
        text_selection: Color::from_rgb(46.0 / 255.0, 112.0 / 255.0, 165.0 / 255.0),
        contact_selected: Color::from_rgb(43.0 / 255.0, 82.0 / 255.0, 120.0 / 255.0),
        contact_hover: Color::from_rgb(32.0 / 255.0, 43.0 / 255.0, 54.0 / 255.0),
        contact: Color::from_rgb(23.0 / 255.0, 33.0 / 255.0, 43.0 / 255.0),
        hovered_bg_scrollbar: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.3),
        hovered_bg_scroller: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.3),
        hovered_bg_scrollbar_mo: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.6),
        hovered_bg_scroller_mo: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.6),
        received_message_bg: Color::from_rgb(24.0 / 255.0, 37.0 / 255.0, 51.0 / 255.0),
        sent_message_bg: Color::from_rgb(43.0 / 255.0, 83.0 / 255.0, 120.0 / 255.0),
        sent_message_status: Color::from_rgb(88.0 / 255.0, 161.0 / 255.0, 217.0 / 255.0),
        sent_message_date: Color::from_rgb(125.0 / 255.0, 168.0 / 255.0, 211.0 / 255.0),
        chat_divider_bg: Color::from_rgb(43.0 / 255.0, 83.0 / 255.0, 120.0 / 255.0),
        chat_search_input_bg: Color::from_rgb(36.0 / 255.0, 47.0 / 255.0, 61.0 / 255.0),
        status_bar_bg: Color::from_rgb(25.0 / 255.0, 26.0 / 255.0, 33.0 / 255.0),
        hover_status_bar_bg: Color::from_rgb(52.0 / 255.0, 53.0 / 255.0, 59.0 / 255.0),
    };
    pub const DARK: Self = Self {
        background: Color::from_rgb(23.0 / 255.0, 33.0 / 255.0, 43.0 / 255.0),
        chat_bg: Color::from_rgb(14.0 / 255.0, 22.0 / 255.0, 33.0 / 255.0),
        send_button: Color::from_rgb(82.0 / 255.0, 136.0 / 255.0, 193.0 / 255.0),
        text_color: Color::from_rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),
        link_1: Color::from_rgb(106.0 / 255.0, 179.0 / 255.0, 243.0 / 255.0),
        link_2: Color::from_rgb(115.0 / 255.0, 185.0 / 255.0, 245.0 / 255.0),
        icons: Color::from_rgb(108.0 / 255.0, 120.0 / 255.0, 131.0 / 255.0),
        placeholder: Color::from_rgb(127.0 / 255.0, 145.0 / 255.0, 164.0 / 255.0),
        text_selection: Color::from_rgb(46.0 / 255.0, 112.0 / 255.0, 165.0 / 255.0),
        contact_selected: Color::from_rgb(43.0 / 255.0, 82.0 / 255.0, 120.0 / 255.0),
        contact_hover: Color::from_rgb(32.0 / 255.0, 43.0 / 255.0, 54.0 / 255.0),
        contact: Color::from_rgb(23.0 / 255.0, 33.0 / 255.0, 43.0 / 255.0),
        hovered_bg_scrollbar: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.1),
        hovered_bg_scroller: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.2),
        hovered_bg_scrollbar_mo: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.2),
        hovered_bg_scroller_mo: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.5),
        received_message_bg: Color::from_rgb(24.0 / 255.0, 37.0 / 255.0, 51.0 / 255.0),
        sent_message_bg: Color::from_rgb(43.0 / 255.0, 83.0 / 255.0, 120.0 / 255.0),
        sent_message_status: Color::from_rgb(88.0 / 255.0, 161.0 / 255.0, 217.0 / 255.0),
        sent_message_date: Color::from_rgb(125.0 / 255.0, 168.0 / 255.0, 211.0 / 255.0),
        chat_divider_bg: Color::from_rgb(30.0 / 255.0, 44.0 / 255.0, 58.0 / 255.0),
        chat_search_input_bg: Color::from_rgb(36.0 / 255.0, 47.0 / 255.0, 61.0 / 255.0),
        status_bar_bg: Color::from_rgb(25.0 / 255.0, 26.0 / 255.0, 33.0 / 255.0),
        hover_status_bar_bg: Color::from_rgb(52.0 / 255.0, 53.0 / 255.0, 59.0 / 255.0),
    };
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.pallete().background.into(),
            text_color: self.pallete().text_color.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Text {
    #[default]
    Default,
    Primary,
    ChatMessageStatus,
    ChatMessageDate,
    Placeholder,

    RelayStatusInitialized,
    RelayStatusConnected,
    RelayStatusConnecting,
    RelayStatusDisconnected,
    RelayStatusTerminated,
    RelayStatusLoading,
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => {
                text::Appearance {
                    color: self.pallete().text_color.into(),
                    // color: color!(0xeb, 0xdb, 0xb2).into(),
                }
            }
            Text::Primary => text::Appearance {
                color: self.pallete().send_button.into(),
            },
            Text::ChatMessageStatus => text::Appearance {
                color: self.pallete().sent_message_status.into(),
            },
            Text::ChatMessageDate => text::Appearance {
                color: self.pallete().sent_message_date.into(),
            },
            Text::Placeholder => text::Appearance {
                color: self.pallete().placeholder.into(),
            },
            Text::RelayStatusInitialized => text::Appearance {
                color: Color::from_rgb8(76, 175, 80).into(),
            },
            Text::RelayStatusConnected => text::Appearance {
                color: Color::from_rgb8(27, 94, 32).into(),
            },
            Text::RelayStatusConnecting => text::Appearance {
                color: Color::from_rgb8(255, 235, 59).into(),
            },
            Text::RelayStatusDisconnected => text::Appearance {
                color: Color::from_rgb8(255, 152, 0).into(),
            },
            Text::RelayStatusTerminated => text::Appearance {
                color: Color::from_rgb8(229, 57, 53).into(),
            },
            Text::RelayStatusLoading => text::Appearance {
                color: Color::from_rgb8(220, 220, 220).into(),
            },
        }
    }
}

impl checkbox::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: if is_checked {
                self.pallete().text_color.into()
            } else {
                self.pallete().background.into()
            },
            icon_color: self.pallete().background,
            border_radius: 4.0,
            border_width: 1.0,
            border_color: self.pallete().text_color,
            text_color: self.pallete().text_color.into(),
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: if is_checked {
                self.pallete().send_button.into()
            } else {
                self.pallete().background.into()
            },
            ..self.active(style, is_checked)
        }
    }
}

impl radio::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style, is_selected: bool) -> radio::Appearance {
        radio::Appearance {
            background: self.pallete().background.into(),
            dot_color: if is_selected {
                self.pallete().text_color
            } else {
                self.pallete().background.into()
            },
            border_width: 1.0,
            border_color: self.pallete().text_color,
            text_color: self.pallete().text_color.into(),
        }
    }

    fn hovered(&self, style: &Self::Style, is_selected: bool) -> radio::Appearance {
        radio::Appearance {
            background: self.pallete().background.into(),
            ..self.active(style, is_selected)
        }
    }
}

const SCROLLABLE_BORDER_RADIUS: f32 = 4.0;
impl scrollable::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Color::TRANSPARENT.into(),
            border_radius: SCROLLABLE_BORDER_RADIUS,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: Color::TRANSPARENT,
                border_radius: SCROLLABLE_BORDER_RADIUS,
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }
    fn dragging(&self, style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            scroller: scrollable::Scroller {
                color: self.pallete().text_color,
                ..self.hovered(style, true).scroller
            },
            ..self.hovered(style, true)
        }
    }
    fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> scrollable::Scrollbar {
        if is_mouse_over_scrollbar {
            scrollable::Scrollbar {
                background: self.pallete().hovered_bg_scrollbar_mo.into(),
                scroller: scrollable::Scroller {
                    color: self.pallete().hovered_bg_scroller_mo,
                    ..self.active(style).scroller
                },
                ..self.active(style)
            }
        } else {
            scrollable::Scrollbar {
                background: self.pallete().hovered_bg_scrollbar.into(),
                scroller: scrollable::Scroller {
                    color: self.pallete().hovered_bg_scroller,
                    ..self.active(style).scroller
                },
                ..self.active(style)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Container {
    #[default]
    Default,
    Bordered,
    TooltipContainer,
    SentMessage,
    ReceivedMessage,
    ChatContainer,
    ChatDateDivider,
    StatusBar,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        let def = container::Appearance {
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            border_radius: 0.0,
            background: Color::TRANSPARENT.into(),
            text_color: self.pallete().text_color.into(),
        };
        match style {
            Container::Default => def,
            Container::Bordered => container::Appearance {
                border_color: self.pallete().text_color,
                border_width: 1.0,
                border_radius: 4.0,
                ..def
            },
            Container::TooltipContainer => container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(Color::from_rgb8(150, 150, 150).into()),
                border_radius: 10.0,
                ..def
            },
            Container::SentMessage => container::Appearance {
                background: self.pallete().sent_message_bg.into(),
                text_color: Color::WHITE.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ReceivedMessage => container::Appearance {
                background: self.pallete().received_message_bg.into(),
                text_color: Color::WHITE.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ChatContainer => container::Appearance {
                background: self.pallete().chat_bg.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::ChatDateDivider => container::Appearance {
                background: self.pallete().chat_divider_bg.into(),
                text_color: self.pallete().text_color.into(),
                border_radius: 10.0,
                ..def
            },
            Container::StatusBar => container::Appearance {
                background: self.pallete().status_bar_bg.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Button {
    #[default]
    Primary,
    Secondary,
    Invisible,
    ContactCard,
    ActiveContactCard,
    ActiveMenuBtn,
    InactiveMenuBtn,
    StatusBarButton,
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let primary = button::Appearance {
            background: self.pallete().send_button.into(),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            text_color: self.pallete().background,
            shadow_offset: Vector { x: 0., y: 0. },
        };
        match style {
            Button::Primary => primary,
            Button::Secondary => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: self.pallete().send_button,
                text_color: self.pallete().send_button,
                ..primary
            },
            Button::Invisible => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: Color::TRANSPARENT,
                text_color: self.pallete().text_color,
                ..primary
            },
            Button::ContactCard => button::Appearance {
                background: self.pallete().contact.into(),
                text_color: self.pallete().text_color,
                border_radius: 0.0,
                ..primary
            },
            Button::ActiveContactCard => button::Appearance {
                border_radius: 0.0,
                background: self.pallete().contact_selected.into(),
                text_color: self.pallete().text_color,
                ..primary
            },
            Button::ActiveMenuBtn => button::Appearance {
                background: self.pallete().contact_selected.into(),
                text_color: self.pallete().text_color,
                border_radius: 5.0,
                ..primary
            },
            Button::InactiveMenuBtn => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.pallete().text_color,
                border_radius: 5.0,
                ..primary
            },
            Button::StatusBarButton => button::Appearance {
                background: self.pallete().status_bar_bg.into(),
                text_color: self.pallete().text_color,
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                ..primary
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => self.active(style),
            Button::Secondary => self.active(style),
            Button::Invisible => self.active(style),
            Button::ContactCard => button::Appearance {
                background: self.pallete().contact_hover.into(),
                text_color: self.pallete().text_color,
                ..self.active(style)
            },
            Button::ActiveContactCard => self.active(style),
            Button::ActiveMenuBtn => button::Appearance {
                ..self.active(style)
            },
            Button::InactiveMenuBtn => button::Appearance {
                background: self.pallete().contact_hover.into(),
                text_color: self.pallete().text_color,
                ..self.active(style)
            },
            Button::StatusBarButton => button::Appearance {
                background: self.pallete().hover_status_bar_bg.into(),
                ..self.active(style)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextInput {
    #[default]
    Default,
    ChatSearch,
}

impl text_input::StyleSheet for Theme {
    type Style = TextInput;
    fn active(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                background: self.pallete().background.into(),
                border_color: self.pallete().background,
                border_radius: 4.0,
                border_width: 1.0,
                icon_color: self.pallete().icons,
            },
            TextInput::ChatSearch => text_input::Appearance {
                background: self.pallete().chat_search_input_bg.into(),
                border_color: self.pallete().background,
                border_radius: 4.0,
                border_width: 0.0,
                icon_color: self.pallete().icons,
            },
        }
    }
    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            // border_color: self.pallete().text_color,
            ..self.active(style)
        }
    }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            // background: self.pallete().background.into(),
            ..self.active(style)
        }
    }
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    fn value_color(&self, _style: &Self::Style) -> Color {
        self.pallete().text_color
    }
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        self.pallete().icons
    }
    fn selection_color(&self, _style: &Self::Style) -> Color {
        self.pallete().text_selection
    }
    fn disabled_color(&self, _style: &Self::Style) -> Color {
        self.pallete().icons
    }
}

// --- ICED AW STYLES ---
#[derive(Debug, Clone, Copy, Default)]
pub enum Modal {
    #[default]
    Default,
}
impl iced_aw::modal::StyleSheet for Theme {
    type Style = Modal;
    fn active(&self, _style: Self::Style) -> iced_aw::style::modal::Appearance {
        iced_aw::style::modal::Appearance {
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Card {
    #[default]
    Default,
}
impl iced_aw::card::StyleSheet for Theme {
    type Style = Card;
    fn active(&self, _style: Self::Style) -> iced_aw::card::Appearance {
        iced_aw::card::Appearance {
            background: Color::from_rgba8(0, 0, 0, 0.5).into(),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            head_background: self.pallete().chat_search_input_bg.into(),
            head_text_color: self.pallete().text_color.into(),
            body_background: self.pallete().background.into(),
            body_text_color: self.pallete().text_color,
            foot_background: self.pallete().background.into(),
            foot_text_color: self.pallete().text_color,
            close_color: self.pallete().text_color,
        }
    }
}

impl iced_aw::split::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> iced_aw::split::Appearance {
        iced_aw::split::Appearance {
            background: None,
            first_background: None,
            second_background: None,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            divider_background: Color::BLACK.into(),
            divider_border_width: 0.0,
            divider_border_color: Color::TRANSPARENT,
        }
    }

    fn hovered(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }

    fn dragged(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }
}
