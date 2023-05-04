use super::Theme;
use iced::widget::text_input;
use iced::Color;

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
