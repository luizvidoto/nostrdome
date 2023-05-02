use crate::{components::text::title, net::events::Event, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(Event),
}

#[derive(Debug, Clone)]
pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::BackEndEvent(_ev) => (),
        }
    }

    pub fn view(&self) -> Element<Message> {
        title("About").into()
    }
}
