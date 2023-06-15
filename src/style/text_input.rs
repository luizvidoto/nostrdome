use super::Theme;
use iced::widget::text_input;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum TextInput {
    #[default]
    Default,
    ChatSearch,
    Invalid,
    Invisible,
}

impl text_input::StyleSheet for Theme {
    type Style = TextInput;

    fn active(&self, style: &Self::Style) -> text_input::Appearance {
        let def = text_input::Appearance {
            background: self.palette().base.background.into(),
            border_radius: 8.0,
            border_width: 1.0,
            border_color: self.palette().base.foreground,
            icon_color: self.palette().base.background,
        };

        let chat_src = text_input::Appearance {
            background: self.palette().base.background.into(),
            border_radius: 8.0,
            border_width: 1.0,
            border_color: self.palette().base.foreground,
            icon_color: self.palette().base.foreground,
        };

        match style {
            TextInput::Default => def,
            TextInput::ChatSearch => chat_src,
            TextInput::Invalid => text_input::Appearance {
                border_color: self.palette().normal.error,
                icon_color: self.palette().normal.error,
                ..chat_src
            },
            TextInput::Invisible => text_input::Appearance {
                background: Color::TRANSPARENT.into(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_color: self.palette().base.background,
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let def = text_input::Appearance {
            background: self.palette().base.foreground.into(),
            border_radius: 8.0,
            border_width: 1.2,
            border_color: self.palette().normal.primary,
            icon_color: self.palette().base.foreground,
        };
        match style {
            TextInput::Default => def,
            TextInput::ChatSearch => def,
            TextInput::Invalid => self.active(style),
            TextInput::Invisible => self.active(style),
        }
    }

    fn disabled(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: self.palette().base.foreground.into(),
            border_radius: 8.0,
            border_width: 1.0,
            border_color: self.palette().base.foreground,
            icon_color: self.palette().base.foreground,
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        self.palette().base.comment
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        self.palette().base.text
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        self.palette().normal.secondary
    }

    /// Produces the style of an hovered text input.
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.focused(style)
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        self.palette().base.comment
    }
}
