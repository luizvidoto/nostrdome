use super::Theme;
use iced::widget::checkbox;

#[derive(Debug, Clone, Copy, Default)]
pub enum Checkbox {
    #[default]
    Default,
}

impl checkbox::StyleSheet for Theme {
    type Style = Checkbox;

    fn active(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        match style {
            Checkbox::Default => checkbox::Appearance {
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
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        match style {
            Checkbox::Default => checkbox::Appearance {
                background: if is_checked {
                    self.pallete().primary.into()
                } else {
                    self.pallete().background.into()
                },
                ..self.active(style, is_checked)
            },
        }
    }
}
