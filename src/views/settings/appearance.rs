use iced::widget::{column, radio};

use crate::{
    components::text::title,
    style::{self, AppPalette, Theme},
    widget::Element,
};

#[derive(Debug, Clone)]
pub enum Message {
    ChangeTheme(style::Theme),
}
pub fn view(selected_theme: Option<style::Theme>) -> Element<'static, Message> {
    let title = title("Appearance");
    let radio_buttons = column![
        radio(
            "Light",
            RadioTheme::from(style::Theme::Light),
            selected_theme.map(RadioTheme::from),
            |radio_theme| Message::ChangeTheme(radio_theme.into())
        ),
        radio(
            "Dark",
            RadioTheme::from(style::Theme::Dark),
            selected_theme.map(RadioTheme::from),
            |radio_theme| Message::ChangeTheme(radio_theme.into())
        ),
    ];

    column![title, radio_buttons].spacing(10).into()
}

#[derive(Debug, Clone, Copy)]
pub struct RadioTheme(u8);

impl PartialEq for RadioTheme {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for RadioTheme {}

impl From<Theme> for RadioTheme {
    fn from(theme: Theme) -> Self {
        let theme_num: u8 = theme.into();
        Self(theme_num)
    }
}

impl Into<Theme> for RadioTheme {
    fn into(self) -> Theme {
        match self.0 {
            0 => Theme::Light,
            1 => Theme::Dark,
            2 => Theme::Custom(AppPalette::default()), // Just an example, replace this with actual custom palette
            _ => unreachable!(),                       // Can only be 0, 1, or 2
        }
    }
}
