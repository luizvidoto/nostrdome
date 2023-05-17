use super::Theme;
use iced::widget::text;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Text {
    #[default]
    Default,
    Inverted,
    Primary,
    Danger,
    ChatMessageStatus,
    ChatMessageDate,
    Placeholder,

    RelayStatusInitialized,
    RelayStatusConnected,
    RelayStatusConnecting,
    RelayStatusDisconnected,
    RelayStatusTerminated,
    RelayStatusLoading,
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => text::Appearance { color: None },
            Text::Inverted => text::Appearance {
                color: self.pallete().background.into(),
            },
            Text::Primary => text::Appearance {
                color: self.pallete().primary.into(),
            },
            Text::Danger => text::Appearance {
                color: self.pallete().danger.into(),
            },
            Text::ChatMessageStatus => text::Appearance {
                color: self.pallete().sent_message_status.into(),
            },
            Text::ChatMessageDate => text::Appearance {
                color: self.pallete().sent_message_date.into(),
            },
            Text::Placeholder => text::Appearance {
                color: self.pallete().placeholder.into(),
            },
            Text::RelayStatusInitialized => text::Appearance {
                color: Color::from_rgb8(76, 175, 80).into(),
            },
            Text::RelayStatusConnected => text::Appearance {
                color: Color::from_rgb8(27, 94, 32).into(),
            },
            Text::RelayStatusConnecting => text::Appearance {
                color: Color::from_rgb8(255, 235, 59).into(),
            },
            Text::RelayStatusDisconnected => text::Appearance {
                color: Color::from_rgb8(255, 152, 0).into(),
            },
            Text::RelayStatusTerminated => text::Appearance {
                color: Color::from_rgb8(229, 57, 53).into(),
            },
            Text::RelayStatusLoading => text::Appearance {
                color: Color::from_rgb8(220, 220, 220).into(),
            },
        }
    }
}
