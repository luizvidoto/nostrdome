use super::{ColorPalette, Theme};
use iced::widget::text;
use iced::Color;
use ns_client::RelayStatus;

#[derive(Debug, Clone, Copy, Default)]
pub enum Text {
    #[default]
    Default,
    Inverted,
    Normal,
    Primary,
    Danger,
    Placeholder,
    Color(Color),
    Alpha(f32),
    RelayStatus(Option<RelayStatus>),
}
impl Text {
    fn relay_status(palette: ColorPalette, status: Option<RelayStatus>) -> text::Appearance {
        match status {
            // Text::RelayStatusInitialized => text::Appearance {
            //     color: Color::from_rgb8(76, 175, 80).into(),
            // },
            Some(RelayStatus::Disconnected) => text::Appearance {
                color: palette.normal.secondary.into(),
            },
            Some(RelayStatus::Connected) => text::Appearance {
                color: palette.normal.success.into(),
            },
            Some(RelayStatus::Connecting) => text::Appearance {
                color: palette.normal.primary.into(),
            },
            Some(RelayStatus::Terminated) => text::Appearance {
                color: palette.normal.error.into(),
            },
            None => text::Appearance {
                color: palette.base.text.into(),
            },
        }
    }
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => text::Appearance::default(),
            Text::Inverted => text::Appearance {
                color: self.palette().base.background.into(),
            },
            Text::Normal => text::Appearance {
                color: self.palette().base.text.into(),
            },
            Text::Primary => text::Appearance {
                color: self.palette().normal.primary.into(),
            },
            Text::Danger => text::Appearance {
                color: self.palette().normal.error.into(),
            },
            Text::Placeholder => text::Appearance {
                color: self.palette().base.comment.into(),
            },
            Text::Color(c) => text::Appearance { color: Some(c) },
            Text::Alpha(a) => {
                let mut color = self.palette().base.text;
                color.a = a;
                text::Appearance {
                    color: color.into(),
                }
            }
            Text::RelayStatus(status_opt) => Text::relay_status(self.palette(), status_opt),
        }
    }
}
