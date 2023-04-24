use iced::widget::{button, checkbox, container, radio, scrollable, text, text_input};
use iced::{application, color, Color, Vector};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppPalette {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub text_light: Color,
    pub text_dark: Color,
    pub background: Color,
}
impl AppPalette {
    pub const LIGHT: Self = Self {
        primary: Color::from_rgb(9. / 255.0, 57. / 255.0, 209. / 255.0),
        secondary: Color::from_rgb(50. / 255.0, 94. / 255.0, 234. / 255.0),
        tertiary: Color::WHITE,
        text_dark: Color::BLACK,
        text_light: Color::WHITE,
        background: Color::WHITE,
    };
    pub const DARK: Self = Self {
        primary: Color::from_rgb(57. / 255.0, 9. / 255.0, 209. / 255.0),
        secondary: Color::from_rgb(94. / 255.0, 50. / 255.0, 234. / 255.0),
        tertiary: Color::WHITE,
        text_dark: Color::WHITE,
        text_light: Color::BLACK,
        background: Color::from_rgb(30. / 255.0, 30. / 255.0, 30. / 255.0),
    };
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.pallete().background.into(),
            text_color: self.pallete().text_dark.into(),
            // background_color: color!(0x28, 0x28, 0x28),
            // text_color: color!(0xeb, 0xdb, 0xb2),
        }
    }
}

impl text::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> text::Appearance {
        text::Appearance {
            color: self.pallete().text_dark.into(),
            // color: color!(0xeb, 0xdb, 0xb2).into(),
        }
    }
}

impl checkbox::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: if is_checked {
                self.pallete().primary.into()
            } else {
                self.pallete().background.into()
            },
            icon_color: self.pallete().background,
            border_radius: 4.0,
            border_width: 1.0,
            border_color: self.pallete().primary,
            text_color: self.pallete().text_dark.into(),
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: if is_checked {
                self.pallete().secondary.into()
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
                self.pallete().primary
            } else {
                self.pallete().background.into()
            },
            border_width: 1.0,
            border_color: self.pallete().primary,
            text_color: self.pallete().text_dark.into(),
        }
    }

    fn hovered(&self, style: &Self::Style, is_selected: bool) -> radio::Appearance {
        radio::Appearance {
            background: self.pallete().secondary.into(),
            ..self.active(style, is_selected)
        }
    }
}

impl scrollable::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Color::WHITE.into(),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::BLACK,
            scroller: scrollable::Scroller {
                color: Color::from_rgb8(9, 57, 209),
                border_radius: 4.0,
                border_width: 1.0,
                border_color: Color::BLACK,
            },
        }
    }
    fn active_horizontal(&self, style: &Self::Style) -> scrollable::Scrollbar {
        self.active(style)
    }
    fn dragging(&self, style: &Self::Style) -> scrollable::Scrollbar {
        self.active(style)
    }
    fn dragging_horizontal(&self, style: &Self::Style) -> scrollable::Scrollbar {
        self.active(style)
    }
    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        self.active(style)
    }
    fn hovered_horizontal(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        self.active(style)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Container {
    #[default]
    Default,
    Bordered,
    TooltipContainer,
    Green,
    Red,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        let def = container::Appearance {
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            border_radius: 0.0,
            background: self.pallete().background.into(),
            text_color: self.pallete().text_dark.into(),
        };
        match style {
            Container::Default => def,
            Container::Bordered => container::Appearance {
                border_color: color!(0x45, 0x85, 0x88),
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
            Container::Green => container::Appearance {
                background: Color::from_rgb8(20, 200, 20).into(),
                text_color: Color::WHITE.into(),
                ..def
            },
            Container::Red => container::Appearance {
                background: Color::from_rgb8(200, 20, 20).into(),
                text_color: Color::WHITE.into(),
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
    ContactCard,
    ActiveContactCard,
    ActiveMenuBtn,
    InactiveMenuBtn,
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let primary = button::Appearance {
            background: self.pallete().primary.into(),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: self.pallete().primary.into(),
            // border_color: color!(0x45, 0x85, 0x88),
            text_color: self.pallete().text_light.into(),
            shadow_offset: Vector { x: 0., y: 0. },
        };
        match style {
            Button::Primary => primary,
            Button::Secondary => button::Appearance {
                background: color!(0x3c, 0x38, 0x36).into(),
                ..primary
            },
            Button::ContactCard => button::Appearance {
                text_color: Color::WHITE,
                border_radius: 0.0,
                background: Some(Color::TRANSPARENT.into()),
                ..primary
            },
            Button::ActiveContactCard => button::Appearance {
                border_radius: 0.0,
                background: Some(Color::from_rgb8(65, 159, 217).into()),
                text_color: Color::WHITE,
                ..primary
            },
            Button::ActiveMenuBtn => button::Appearance {
                text_color: Color::WHITE,
                border_radius: 5.0,
                background: Some(Color::from_rgb8(65, 159, 217).into()),
                ..primary
            },
            Button::InactiveMenuBtn => button::Appearance {
                text_color: Color::BLACK,
                border_radius: 5.0,
                background: Some(Color::TRANSPARENT.into()),
                ..primary
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => self.active(style),
            Button::Secondary => button::Appearance {
                ..self.active(style)
            },
            Button::ContactCard => button::Appearance {
                background: Some(Color::from_rgb8(240, 240, 240).into()),
                text_color: Color::BLACK,
                ..self.active(style)
            },
            Button::ActiveContactCard => self.active(style),
            Button::ActiveMenuBtn => self.active(style),
            Button::InactiveMenuBtn => button::Appearance {
                background: Some(Color::from_rgb8(240, 240, 240).into()),
                text_color: Color::BLACK,
                ..self.active(style)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextInput {
    #[default]
    Default,
}

impl text_input::StyleSheet for Theme {
    type Style = TextInput;
    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Color::WHITE.into(),
            border_color: Color::BLACK,
            border_radius: 4.0,
            border_width: 1.0,
            icon_color: Color::BLACK,
        }
    }
    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            border_color: Color::from_rgb8(9, 57, 209),
            icon_color: Color::from_rgb8(9, 57, 209),
            ..self.active(style)
        }
    }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Color::from_rgb8(139, 139, 139).into(),
            icon_color: Color::from_rgb8(191, 191, 191),
            ..self.active(style)
        }
    }
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    fn value_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb8(9, 57, 209)
    }
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb8(181, 181, 181)
    }
    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb8(9, 57, 209)
    }
    fn disabled_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb8(191, 191, 191)
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
            ..Default::default()
        }
    }
}

impl iced_aw::split::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> iced_aw::split::Appearance {
        iced_aw::split::Appearance {
            ..Default::default()
        }
    }

    fn hovered(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }

    fn dragged(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }
}
