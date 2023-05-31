#![allow(unused_imports)]
use iced::{application, widget::tooltip};
use iced_style::{rule::FillMode, Color};

use self::pallete::AppPalette;

mod button;
mod card;
mod checkbox;
mod container;
mod modal;
mod pallete;
mod radio;
mod scrollable;
mod slider;
mod split;
mod text;
mod text_input;

pub(crate) use button::Button;
pub(crate) use card::Card;
pub(crate) use checkbox::Checkbox;
pub(crate) use container::Container;
pub(crate) use modal::Modal;
pub(crate) use radio::Radio;
pub(crate) use scrollable::Scrollable;
pub(crate) use slider::Slider;
pub(crate) use split::Split;
pub(crate) use text::Text;
pub(crate) use text_input::TextInput;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}
impl Theme {
    pub fn pallete(&self) -> AppPalette {
        match self {
            Theme::Light => AppPalette::LIGHT,
            Theme::Dark => AppPalette::DARK,
        }
    }
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.pallete().background,
            text_color: self.pallete().text_color,
        }
    }
}

impl iced::widget::rule::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> iced_style::rule::Appearance {
        iced_style::rule::Appearance {
            color: self.pallete().placeholder.into(),
            width: 1,
            radius: 2.0,
            fill_mode: FillMode::Full,
        }
    }
}
