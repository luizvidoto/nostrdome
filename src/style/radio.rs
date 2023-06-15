use super::Theme;
use iced::widget::radio;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Radio {
    #[default]
    Primary,
}

impl radio::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style, _is_selected: bool) -> radio::Appearance {
        radio::Appearance {
            background: Color::TRANSPARENT.into(),
            dot_color: self.palette().normal.primary,
            border_width: 1.0,
            border_color: self.palette().normal.primary,
            text_color: None,
        }
    }

    fn hovered(&self, style: &Self::Style, _is_selected: bool) -> radio::Appearance {
        let active = self.active(style, true);

        radio::Appearance {
            dot_color: self.palette().normal.primary,
            border_color: self.palette().normal.primary,
            border_width: 2.0,
            ..active
        }
    }
}
