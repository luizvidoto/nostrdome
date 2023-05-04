use super::Theme;
use iced::widget::radio;

#[derive(Debug, Clone, Copy, Default)]
pub enum Radio {
    #[default]
    Default,
}

impl radio::StyleSheet for Theme {
    type Style = Radio;

    fn active(&self, style: &Self::Style, is_selected: bool) -> radio::Appearance {
        match style {
            Radio::Default => radio::Appearance {
                background: self.pallete().background.into(),
                dot_color: if is_selected {
                    self.pallete().text_color
                } else {
                    self.pallete().background.into()
                },
                border_width: 1.0,
                border_color: self.pallete().text_color,
                text_color: self.pallete().text_color.into(),
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_selected: bool) -> radio::Appearance {
        match style {
            Radio::Default => radio::Appearance {
                background: self.pallete().background.into(),
                ..self.active(style, is_selected)
            },
        }
    }
}
