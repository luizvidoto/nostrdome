use iced::Element;

use crate::components::text::title;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub struct State {}
impl State {
    pub fn default() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) {
        match message {}
    }

    pub fn view(&self) -> Element<Message> {
        title("Backup").into()
    }
}
