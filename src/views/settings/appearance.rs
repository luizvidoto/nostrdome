use iced::widget::{column, radio};

use crate::{components::text::title, style, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    ChangeTheme(style::Theme),
}
pub fn view(selected_theme: Option<style::Theme>) -> Element<'static, Message> {
    let title = title("Appearance");
    let radio_buttons = column![
        radio(
            "Light",
            style::Theme::Light,
            selected_theme,
            Message::ChangeTheme
        ),
        radio(
            "Dark",
            style::Theme::Dark,
            selected_theme,
            Message::ChangeTheme
        ),
    ];

    column![title, radio_buttons].spacing(10).into()
}
