use crate::components::modal;

use super::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub enum Modal {
    #[default]
    Default,
}
impl iced_aw::modal::StyleSheet for Theme {
    type Style = Modal;
    fn active(&self, _style: Self::Style) -> iced_aw::style::modal::Appearance {
        iced_aw::style::modal::Appearance {
            ..Default::default()
        }
    }
}

impl modal::StyleSheet for Theme {
    type Style = Modal;
    fn active(&self, _style: Self::Style) -> modal::Appearance {
        modal::Appearance {
            ..Default::default()
        }
    }
}
