use crate::utils::{change_color_by_type, darken_color, lighten_color};

use super::Theme;
use iced::widget::container;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Container {
    #[default]
    Default,
    Background,
    Foreground,
    ForegroundBordered,
    Bordered,
    CardBody,
    Frame,
    SentMessage,
    ReceivedMessage,
    ChatDateDivider,
    StatusBar,
    TooltipBg,
    ContextMenu,
    WelcomeBg1,
    WelcomeBg2,
    WelcomeBg3,
    WithColor(Color),
    CardFoot,
    Highlight,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        let def = container::Appearance::default();
        let theme_type = self.theme_meta().theme_type;

        let frame = container::Appearance {
            background: self.palette().base.foreground.into(),
            text_color: self.palette().base.text.into(),
            border_radius: 5.0,
            ..def
        };

        let modal = container::Appearance {
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            border_radius: 10.0,
            background: self.palette().base.background.into(),
            text_color: None,
        };

        match style {
            Container::Default => def,
            Container::Background => container::Appearance {
                background: self.palette().base.background.into(),
                text_color: self.palette().base.text.into(),
                ..def
            },
            Container::Highlight => container::Appearance {
                background: darken_color(self.palette().normal.primary, 0.1).into(),
                text_color: self.palette().base.text.into(),
                ..def
            },
            Container::Foreground => container::Appearance {
                background: self.palette().base.foreground.into(),
                text_color: self.palette().base.text.into(),
                ..def
            },
            Container::ForegroundBordered => container::Appearance {
                background: self.palette().base.foreground.into(),
                border_width: 1.0,
                border_color: change_color_by_type(theme_type, self.palette().base.background, 0.1),
                text_color: self.palette().base.text.into(),
                ..def
            },
            Container::Bordered => container::Appearance {
                border_color: change_color_by_type(theme_type, self.palette().base.foreground, 0.1),
                border_width: 1.0,
                border_radius: 4.0,
                ..def
            },
            Container::Frame => frame,
            Container::CardBody => modal,
            Container::CardFoot => container::Appearance {
                background: self.palette().base.foreground.into(),
                ..modal
            },
            Container::SentMessage => container::Appearance {
                background: self.palette().normal.primary_variant.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ReceivedMessage => container::Appearance {
                background: self.palette().base.foreground.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ChatDateDivider => container::Appearance {
                background: self.palette().base.foreground.into(),
                text_color: self.palette().base.text.into(),
                border_radius: 10.0,
                ..def
            },
            Container::StatusBar => container::Appearance {
                background: self.palette().base.background.into(),
                text_color: self.palette().base.text.into(),
                ..def
            },
            Container::WelcomeBg1 => container::Appearance {
                background: Color::BLACK.into(),
                text_color: Color::WHITE.into(),
                ..def
            },
            Container::WelcomeBg2 => container::Appearance {
                background: Color::BLACK.into(),
                text_color: Color::WHITE.into(),
                ..def
            },
            Container::WelcomeBg3 => container::Appearance {
                background: Color::BLACK.into(),
                text_color: Color::WHITE.into(),
                ..def
            },
            Container::TooltipBg => container::Appearance {
                background: Color::from_rgba(0.0, 0.0, 0.0, 0.95).into(),
                text_color: Color::from_rgb8(255, 255, 255).into(),
                border_width: 0.0,
                border_radius: 10.0,
                ..def
            },
            Container::ContextMenu => container::Appearance {
                background: Color::from_rgba(0.0, 0.0, 0.0, 0.97).into(),
                text_color: Color::BLACK.into(),
                border_width: 0.0,
                border_radius: 10.0,
                ..def
            },
            Container::WithColor(color) => container::Appearance {
                background: color.to_owned().into(),
                ..def
            },
        }
    }
}
