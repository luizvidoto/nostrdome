use iced::Element;

use crate::{components::text::title, net};

#[derive(Debug, Clone)]
pub enum Message {
    DbEvent(net::Event),
}

#[derive(Debug, Clone)]
pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::DbEvent(_ev) => (),
        }
    }

    pub fn view(&self) -> Element<Message> {
        title("Appearance").into()
    }
}
