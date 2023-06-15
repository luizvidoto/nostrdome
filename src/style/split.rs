use crate::utils::change_color_by_type;

use super::Theme;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Split {
    #[default]
    Default,
}

impl iced_aw::split::StyleSheet for Theme {
    type Style = Split;

    fn active(&self, _style: Self::Style) -> iced_aw::split::Appearance {
        let theme_type = self.theme_meta().theme_type;

        iced_aw::split::Appearance {
            background: None,
            first_background: None,
            second_background: None,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            divider_background: change_color_by_type(
                theme_type,
                self.palette().base.background,
                0.1,
            )
            .into(),
            divider_border_width: 0.0,
            divider_border_color: Color::TRANSPARENT,
        }
    }

    fn hovered(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }

    fn dragged(&self, style: Self::Style) -> iced_aw::split::Appearance {
        self.active(style)
    }
}
