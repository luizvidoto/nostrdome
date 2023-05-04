use super::Theme;
use iced::Color;

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
