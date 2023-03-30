use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};

use crate::components::text::title;

#[derive(Debug, Clone)]
pub enum RelayMessage {
    None,
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    is_connected: bool,
    address: String,
    last_activity: i64,
    is_read: bool,
    is_write: bool,
    is_advertise: bool,
}
impl RelayRow {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            is_connected: false,
            address: address.into(),
            last_activity: 0,
            is_read: false,
            is_write: false,
            is_advertise: false,
        }
    }
    pub fn view(&self) -> Element<'static, RelayMessage> {
        row![
            text(if self.is_connected {
                "Online"
            } else {
                "Offline"
            })
            .width(Length::Fill),
            container(text(&self.address)).width(Length::Fill),
            container(text(format!("{}s", self.last_activity))).width(Length::Fill),
            container(checkbox("", self.is_read, |_| RelayMessage::None)).width(Length::Fill),
            container(checkbox("", self.is_write, |_| RelayMessage::None)).width(Length::Fill),
            container(checkbox("", self.is_advertise, |_| RelayMessage::None)).width(Length::Fill),
            button("Remove").width(Length::Fill),
        ]
        .into()
    }
    pub fn view_header() -> Element<'static, RelayMessage> {
        row![
            text("Status").width(Length::Fill),
            text("Address").width(Length::Fill),
            text("Last Active").width(Length::Fill),
            text("Read").width(Length::Fill),
            text("Write").width(Length::Fill),
            text("Advertise").width(Length::Fill),
            text("").width(Length::Fill)
        ]
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(RelayMessage),
}

#[derive(Debug, Clone)]
pub struct State {
    relays: Vec<RelayRow>,
}
impl State {
    pub fn new(relays: Vec<RelayRow>) -> Self {
        Self { relays }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::RelayMessage(_msg) => (),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Network");

        let relays = self.relays.iter().fold(
            column![RelayRow::view_header().map(Message::RelayMessage)],
            |col, relay| col.push(relay.view().map(Message::RelayMessage)),
        );
        container(column![title, relays])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
