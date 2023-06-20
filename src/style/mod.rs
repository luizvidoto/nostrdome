#![allow(unused_imports)]
use iced::{
    application,
    widget::{rule, tooltip},
};
use iced_native::color;
use iced_style::{rule::FillMode, Color};

use crate::Error;

mod button;
mod card;
mod checkbox;
mod container;
mod modal;
mod palette;
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
pub(crate) use palette::{ColorPalette, Theme, ThemeMeta, ThemeType};
pub(crate) use radio::Radio;
pub(crate) use scrollable::Scrollable;
pub(crate) use slider::Slider;
pub(crate) use split::Split;
pub(crate) use text::Text;
pub(crate) use text_input::TextInput;

#[derive(Default, Debug, Clone, Copy)]
pub enum Application {
    #[default]
    Default,
}

impl application::StyleSheet for Theme {
    type Style = Application;

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.palette().base.background,
            text_color: self.palette().base.text,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum Rule {
    #[default]
    Default,
}

impl rule::StyleSheet for Theme {
    type Style = Rule;

    fn appearance(&self, style: &Self::Style) -> rule::Appearance {
        match style {
            Rule::Default => rule::Appearance {
                color: self.palette().base.foreground,
                width: 1,
                radius: 1.0,
                fill_mode: rule::FillMode::Full,
            },
        }
    }
}
