#![allow(unused_imports)]
use iced::{application, widget::tooltip};
use iced_style::{rule::FillMode, Color};

pub(crate) use self::pallete::AppPalette;
use crate::Error;

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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    Custom(AppPalette),
}
impl Theme {
    pub fn palette(&self) -> AppPalette {
        match self {
            Theme::Light => AppPalette::LIGHT,
            Theme::Dark => AppPalette::DARK,
            Theme::Custom(pallete) => pallete.to_owned(),
        }
    }
}

impl From<Theme> for u8 {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => 0,
            Theme::Dark => 1,
            Theme::Custom(_) => 2,
        }
    }
}

impl TryInto<Theme> for u8 {
    type Error = Error;
    fn try_into(self) -> Result<Theme, Error> {
        match self {
            0 => Ok(Theme::Light),
            1 => Ok(Theme::Dark),
            2 => Ok(Theme::Custom(AppPalette::default())),
            other => Err(Error::NotFoundTheme(other)),
        }
    }
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.palette().background,
            text_color: self.palette().text_color,
        }
    }
}

impl iced::widget::rule::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> iced_style::rule::Appearance {
        iced_style::rule::Appearance {
            color: self.palette().grayish.into(),
            width: 1,
            radius: 2.0,
            fill_mode: FillMode::Full,
        }
    }
}
