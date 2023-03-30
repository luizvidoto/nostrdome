use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};

use crate::components::text::title;

#[derive(Debug, Clone)]
pub enum RelayMessage {
    None,
    DeleteRelay(String),
    ToggleRead(String),
    ToggleWrite(String),
    ToggleAdvertise(String),
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
    pub fn view<'a>(&'a self) -> Element<'a, RelayMessage> {
        row![
            text(if self.is_connected {
                "Online"
            } else {
                "Offline"
            })
            .width(Length::Fill),
            container(text(&self.address)).width(Length::Fill),
            container(text(format!("{}s", self.last_activity))).width(Length::Fill),
            container(checkbox("", self.is_read, |_| RelayMessage::ToggleRead(
                self.address.clone()
            )))
            .width(Length::Fill),
            container(checkbox("", self.is_write, |_| RelayMessage::ToggleWrite(
                self.address.clone()
            )))
            .width(Length::Fill),
            container(checkbox("", self.is_advertise, |_| {
                RelayMessage::ToggleAdvertise(self.address.clone())
            }))
            .width(Length::Fill),
            button("Remove")
                .on_press(RelayMessage::DeleteRelay(self.address.clone()))
                .width(Length::Fill),
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
    pub fn update(&mut self, message: RelayMessage) {
        match message {
            RelayMessage::None => (),
            RelayMessage::DeleteRelay(_) => (),
            RelayMessage::ToggleRead(_) => self.is_read = !self.is_read,
            RelayMessage::ToggleWrite(_) => self.is_write = !self.is_write,
            RelayMessage::ToggleAdvertise(_) => self.is_advertise = !self.is_advertise,
        }
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
            Message::RelayMessage(msg) => match msg.clone() {
                RelayMessage::None => (),
                RelayMessage::DeleteRelay(_addrs) => (),
                RelayMessage::ToggleRead(addrs)
                | RelayMessage::ToggleWrite(addrs)
                | RelayMessage::ToggleAdvertise(addrs) => {
                    for r in &mut self.relays {
                        if &r.address == &addrs {
                            r.update(msg.clone());
                            break;
                        }
                    }
                }
            },
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
