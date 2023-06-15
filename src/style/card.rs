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
            background: self.palette().base.background.into(),
            border_radius: 4.0,
            border_width: 2.0,
            border_color: Color::TRANSPARENT,
            head_background: self.palette().base.background.into(),
            head_text_color: self.palette().base.text,

            body_background: self.palette().base.foreground.into(),
            body_text_color: self.palette().base.text,

            foot_background: self.palette().base.background.into(),
            foot_text_color: self.palette().base.text,

            close_color: self.palette().base.text,
        }
    }
}
