use iced::alignment::Horizontal;
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};
use iced_aw::{Card, Modal};

use crate::components::text::title;
use crate::components::text_input_group::text_input_group;
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
    OpenAddRelayModal,
    CancelButtonPressed,
    OkButtonPressed,
    CloseModal,
    AddRelayInputChange(String),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Loaded {
        relays: Vec<RelayRow>,
        show_modal: bool,
        add_relay_input: String,
    },
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
        Self::Loaded {
            relays,
            show_modal: false,
            add_relay_input: "".into(),
        }
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
            State::Loaded {
                ref mut relays,
                ref mut show_modal,
                ref mut add_relay_input,
            } => match message {
                Message::AddRelayInputChange(relay_addrs) => *add_relay_input = relay_addrs,
                Message::CloseModal | Message::CancelButtonPressed => {
                    *add_relay_input = "".into();
                    *show_modal = false;
                }
                Message::OkButtonPressed => {
                    match RelayUrl::try_from_str(add_relay_input) {
                        Ok(url) => {
                            if let Err(e) = conn.send(net::Message::AddRelay(DbRelay::new(url))) {
                                tracing::error!("{}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("{}", e);
                        }
                    }
                    *add_relay_input = "".into();
                    *show_modal = false;
                }
                Message::OpenAddRelayModal => *show_modal = true,
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
            State::Loaded { relays, .. } => relays.iter().fold(header, |col, relay| {
                col.push(relay.view().map(Message::RelayMessage))
            }),
        };
        let empty = container(text("")).width(Length::Fill);
        let add_btn = button("Add").on_press(Message::OpenAddRelayModal);
        let add_row = row![empty, add_btn];
        let content = container(column![title, add_row, relays])
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        match self {
            State::Loading => content,
            State::Loaded {
                show_modal,
                add_relay_input,
                ..
            } => {
                Modal::new(*show_modal, content, || {
                    let add_relay_input = text_input_group(
                        "Relay Address",
                        "wss://my-relay.com",
                        add_relay_input,
                        None,
                        Message::AddRelayInputChange,
                    );
                    let modal_body: Element<_> = container(add_relay_input).into();
                    Card::new(text("Add Relay"), modal_body)
                        .foot(
                            row![
                                button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::CancelButtonPressed),
                                button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::OkButtonPressed)
                            ]
                            .spacing(10)
                            .padding(5)
                            .width(Length::Fill),
                        )
                        .max_width(300.0)
                        //.width(Length::Shrink)
                        .on_close(Message::CloseModal)
                        .into()
                })
                .backdrop(Message::CloseModal)
                .on_esc(Message::CloseModal)
                .into()
            }
        }
    }
}
