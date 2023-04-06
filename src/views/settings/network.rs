use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};

use crate::components::text::title;
use crate::db::DbRelay;
use crate::net::{self, Connection};
use crate::types::RelayUrl;

#[derive(Debug, Clone)]
pub enum RelayMessage {
    None,
    DeleteRelay(RelayUrl),
    ToggleRead(DbRelay),
    ToggleWrite(DbRelay),
    ToggleAdvertise(DbRelay),
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    is_connected: bool,
    url: RelayUrl,
    last_connected_at: Option<i32>,
    is_read: bool,
    is_write: bool,
    is_advertise: bool,
}
impl RelayRow {
    pub fn new(relay: &DbRelay) -> Self {
        Self {
            is_connected: false,
            url: relay.url.clone(),
            last_connected_at: relay.last_connected_at,
            is_read: relay.read,
            is_write: relay.write,
            is_advertise: relay.advertise,
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
            container(text(&self.url)).width(Length::Fill),
            container(text(format!("{}s", self.last_connected_at.unwrap_or(0))))
                .width(Length::Fill),
            container(checkbox("", self.is_read, |_| RelayMessage::ToggleRead(
                self.into()
            )))
            .width(Length::Fill),
            container(checkbox("", self.is_write, |_| RelayMessage::ToggleWrite(
                self.into()
            )))
            .width(Length::Fill),
            container(checkbox("", self.is_advertise, |_| {
                RelayMessage::ToggleAdvertise(self.into())
            }))
            .width(Length::Fill),
            button("Remove")
                .on_press(RelayMessage::DeleteRelay(self.url.clone()))
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
}

impl From<&RelayRow> for DbRelay {
    fn from(row: &RelayRow) -> Self {
        Self {
            url: row.url.to_owned(),
            last_connected_at: row.last_connected_at,
            read: row.is_read,
            write: row.is_write,
            advertise: row.is_advertise,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(RelayMessage),
    DbEvent(net::Event),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Loaded { relays: Vec<RelayRow> },
}
impl State {
    pub fn loading(conn: &mut Connection) -> Self {
        if let Err(e) = conn.send(net::Message::FetchRelays) {
            tracing::error!("{}", e);
        }
        Self::Loading
    }
    pub fn loaded(relays: Vec<DbRelay>) -> Self {
        let relays = relays.iter().map(|r| RelayRow::new(r)).collect();
        Self::Loaded { relays }
    }

    pub fn update(&mut self, message: Message, conn: &mut Connection) {
        match self {
            State::Loading => {
                if let Message::DbEvent(ev) = message {
                    if let net::Event::GotDbRelays(db_relays) = ev {
                        *self = Self::loaded(db_relays);
                    }
                }
            }
            State::Loaded { ref mut relays } => match message {
                Message::DbEvent(ev) => match ev {
                    net::Event::GotDbRelays(db_relays) => {
                        *relays = db_relays.iter().map(|r| RelayRow::new(r)).collect()
                    }
                    net::Event::DatabaseSuccessEvent(kind) => match kind {
                        net::DatabaseSuccessEventKind::RelayCreated
                        | net::DatabaseSuccessEventKind::RelayDeleted
                        | net::DatabaseSuccessEventKind::RelayUpdated => {
                            if let Err(e) = conn.send(net::Message::FetchRelays) {
                                tracing::error!("{}", e);
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                },
                Message::RelayMessage(msg) => match msg.clone() {
                    RelayMessage::None => (),
                    RelayMessage::DeleteRelay(relay_url) => {
                        if let Err(e) = conn.send(net::Message::DeleteRelay(relay_url)) {
                            tracing::error!("{}", e);
                        }
                    }
                    RelayMessage::ToggleRead(mut db_relay) => {
                        db_relay.read = !db_relay.read;
                        if let Err(e) = conn.send(net::Message::UpdateRelay(db_relay)) {
                            tracing::error!("{}", e);
                        }
                    }
                    RelayMessage::ToggleWrite(mut db_relay) => {
                        db_relay.write = !db_relay.write;
                        if let Err(e) = conn.send(net::Message::UpdateRelay(db_relay)) {
                            tracing::error!("{}", e);
                        }
                    }
                    RelayMessage::ToggleAdvertise(mut db_relay) => {
                        db_relay.advertise = !db_relay.advertise;
                        if let Err(e) = conn.send(net::Message::UpdateRelay(db_relay)) {
                            tracing::error!("{}", e);
                        }
                    }
                },
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Network");
        let header = column![RelayRow::view_header().map(Message::RelayMessage)];
        let relays = match self {
            State::Loading => header.into(),
            State::Loaded { relays } => relays.iter().fold(header, |col, relay| {
                col.push(relay.view().map(Message::RelayMessage))
            }),
        };

        container(column![title, relays])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
