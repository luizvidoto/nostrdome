use crate::{components::text::title, net::events::Event, style, widget::Element};
use iced::widget::{column, radio};

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(Event),
    ChangeTheme(style::Theme),
}

#[derive(Debug, Clone)]
pub struct State {
    selected_theme: Option<style::Theme>,
}
impl State {
    pub fn new(selected_theme: Option<style::Theme>) -> Self {
        Self { selected_theme }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::BackEndEvent(_ev) => (),
            Message::ChangeTheme(theme) => self.selected_theme = Some(theme),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Appearance");
        let radio_buttons = column![
            radio(
                "Light",
                style::Theme::Light,
                self.selected_theme,
                Message::ChangeTheme
            ),
            radio(
                "Dark",
                style::Theme::Dark,
                self.selected_theme,
                Message::ChangeTheme
            ),
        ];

        column![title, radio_buttons].spacing(10).into()
    }
}
