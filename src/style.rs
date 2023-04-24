use iced::widget::{button, container, text};
use iced::{application, color, Color};

#[derive(Debug, Clone, Copy, Default)]
pub struct Theme;

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: color!(0x28, 0x28, 0x28),
            text_color: color!(0xeb, 0xdb, 0xb2),
        }
    }
}

impl text::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> text::Appearance {
        text::Appearance {
            color: color!(0xeb, 0xdb, 0xb2).into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Container {
    #[default]
    Default,
    Bordered,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            Container::Default => container::Appearance::default(),
            Container::Bordered => container::Appearance {
                border_color: color!(0x45, 0x85, 0x88),
                border_width: 1.0,
                border_radius: 4.0,
                ..Default::default()
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
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => button::Appearance {
                background: color!(0x28, 0x28, 0x28).into(),
                border_radius: 4.0,
                border_width: 1.0,
                border_color: color!(0x45, 0x85, 0x88),
                ..Default::default()
            },
            Button::Secondary => button::Appearance {
                background: color!(0x3c, 0x38, 0x36).into(),
                ..Default::default()
            },
            Button::ContactCard => button::Appearance {
                text_color: Color::WHITE,
                border_radius: 0.0,
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            },
            Button::ActiveContactCard => button::Appearance {
                border_radius: 0.0,
                background: Some(Color::from_rgb8(65, 159, 217).into()),
                text_color: Color::WHITE,
                ..Default::default()
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => button::Appearance {
                ..self.active(style)
            },
            Button::Secondary => button::Appearance {
                ..self.active(style)
            },
            Button::ContactCard => button::Appearance {
                background: Some(Color::from_rgb8(240, 240, 240).into()),
                text_color: Color::BLACK,
                ..self.active(style)
            },
            Button::ActiveContactCard => button::Appearance {
                ..self.active(style)
            },
        }
    }
}
