use super::Theme;
use iced::widget::container;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Container {
    #[default]
    Default,
    Bordered,
    TooltipIcon,
    SentMessage,
    ReceivedMessage,
    ChatContainer,
    ChatDateDivider,
    ContactList,
    ChatSearchCopy,
    StatusBar,
    TooltipBg,
    ContextMenu,
    WelcomeBg1,
    WelcomeBg2,
    WelcomeBg3,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        let def = container::Appearance {
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            border_radius: 0.0,
            background: Color::TRANSPARENT.into(),
            text_color: self.pallete().text_color.into(),
        };
        match style {
            Container::Default => def,
            Container::Bordered => container::Appearance {
                border_color: self.pallete().text_color,
                border_width: 1.0,
                border_radius: 4.0,
                ..def
            },
            Container::ChatSearchCopy => container::Appearance {
                background: self.pallete().chat_search_input_bg.into(),
                border_color: self.pallete().background,
                border_radius: 4.0,
                border_width: 1.0,
                ..def
            },
            Container::TooltipIcon => container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(Color::from_rgb8(150, 150, 150).into()),
                border_radius: 10.0,
                ..def
            },
            Container::SentMessage => container::Appearance {
                background: self.pallete().sent_message_bg.into(),
                text_color: Color::WHITE.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ReceivedMessage => container::Appearance {
                background: self.pallete().received_message_bg.into(),
                text_color: Color::WHITE.into(),
                border_radius: 10.0,
                ..def
            },
            Container::ContactList => container::Appearance {
                background: self.pallete().background.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::ChatContainer => container::Appearance {
                background: self.pallete().chat_bg.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::ChatDateDivider => container::Appearance {
                background: self.pallete().chat_divider_bg.into(),
                text_color: self.pallete().text_color.into(),
                border_radius: 10.0,
                ..def
            },
            Container::StatusBar => container::Appearance {
                background: self.pallete().status_bar_bg.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::WelcomeBg1 => container::Appearance {
                background: self.pallete().welcome_bg_1.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::WelcomeBg2 => container::Appearance {
                background: self.pallete().welcome_bg_2.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::WelcomeBg3 => container::Appearance {
                background: self.pallete().welcome_bg_3.into(),
                text_color: self.pallete().text_color.into(),
                ..def
            },
            Container::TooltipBg => container::Appearance {
                background: Color::from_rgba(0.0, 0.0, 0.0, 0.95).into(),
                text_color: self.pallete().text_color.into(),
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
        }
    }
}
