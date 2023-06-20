use crate::utils::change_color_by_type;

use super::Theme;
use iced::widget::checkbox;
use iced::Color;
use iced_style::Background;

#[derive(Default, Debug, Clone, Copy)]
pub enum Checkbox {
    #[default]
    Normal,
    Inverted,
}

impl checkbox::StyleSheet for Theme {
    type Style = Checkbox;

    fn active(&self, style: &Self::Style, _is_checked: bool) -> checkbox::Appearance {
        let default = checkbox::Appearance {
            background: self.palette().normal.primary.into(),
            icon_color: self.palette().base.text,
            border_radius: 5.0,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            text_color: Some(self.palette().base.text),
        };
        match style {
            Checkbox::Normal => default,
            Checkbox::Inverted => checkbox::Appearance {
                background: self.palette().base.foreground.into(),
                ..default
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        let active = self.active(style, is_checked);
        let theme_type = self.theme_meta().theme_type;

        let background = {
            let color = match active.background {
                Background::Color(color) => change_color_by_type(theme_type, color, 0.05),
            };
            Background::Color(color)
        };

        let from_appearance = checkbox::Appearance {
            background,
            ..active
        };

        match style {
            Checkbox::Normal => from_appearance,
            Checkbox::Inverted => checkbox::Appearance {
                background: self.palette().base.foreground.into(),
                ..from_appearance
            },
        }
    }
}
