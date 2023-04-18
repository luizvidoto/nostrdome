use crate::db::{DbRelay, DbRelayStatus};
use crate::error::Error;
use crate::net::{self, BackEndConnection};
use crate::types::RelayUrl;
use iced::widget::{button, checkbox, container, row, text};
use iced::{Command, Element, Length, Subscription};
use iced_native::futures::channel::mpsc;
use nostr_sdk::Relay;

#[derive(Debug, Clone)]
pub struct RelayRowConnection(mpsc::UnboundedSender<Input>);
impl RelayRowConnection {
    pub fn send(&mut self, input: Input) {
        if let Err(e) = self.0.unbounded_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConnectToRelay(RelayUrl),
    UpdateStatus((RelayUrl, DbRelayStatus, i64)),
    DeleteRelay(RelayUrl),
    ToggleRead(DbRelay),
    ToggleWrite(DbRelay),
    ToggleAdvertise(DbRelay),
    Ready(RelayRowConnection),
}

pub enum Input {
    GetStatus,
}
pub enum State {
    Idle {
        url: RelayUrl,
        relay: Relay,
    },
    Querying {
        url: RelayUrl,
        receiver: mpsc::UnboundedReceiver<Input>,
        relay: Relay,
    },
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    url: RelayUrl,
    status: DbRelayStatus,
    last_connected_at: Option<i64>,
    is_read: bool,
    is_write: bool,
    _is_advertise: bool,
    relay: Relay,
}

impl RelayRow {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::subscription::unfold(
            self.url.clone(),
            State::Idle {
                url: self.url.clone(),
                relay: self.relay.clone(),
            },
            |state| async move {
                match state {
                    State::Idle { relay, url } => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        let (sender, receiver) = mpsc::unbounded();
                        (
                            Message::Ready(RelayRowConnection(sender)),
                            State::Querying {
                                url,
                                relay,
                                receiver,
                            },
                        )
                    }
                    State::Querying {
                        url,
                        relay,
                        mut receiver,
                    } => {
                        use iced_native::futures::StreamExt;

                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let input = receiver.select_next_some().await;

                        let (relay_status, last_connected_at) = match input {
                            Input::GetStatus => {
                                (relay.status().await, relay.stats().connected_at())
                            }
                        };

                        (
                            Message::UpdateStatus((
                                url.clone(),
                                DbRelayStatus::new(relay_status),
                                last_connected_at.as_i64(),
                            )),
                            State::Idle { url, relay },
                        )
                    }
                }
            },
        )
    }
    pub fn into_db_relay(&self) -> DbRelay {
        DbRelay {
            url: self.url.clone(),
            read: self.is_read,
            write: self.is_write,
            last_connected_at: self.last_connected_at,
            status: self.status.clone(),
            advertise: self._is_advertise,
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match message {
            Message::None => (),
            Message::ConnectToRelay(url) => {
                back_conn.send(net::Message::ConnectToRelay(url));
            }
            Message::UpdateStatus((url, status, last_connected_at)) => {
                if self.url == url {
                    self.status = status;
                    self.last_connected_at = Some(last_connected_at);
                    back_conn.send(net::Message::UpdateRelay(self.into_db_relay()));
                }
            }
            Message::DeleteRelay(relay_url) => {
                back_conn.send(net::Message::DeleteRelay(relay_url));
            }
            Message::ToggleRead(mut db_relay) => {
                db_relay.read = !db_relay.read;
                back_conn.send(net::Message::UpdateRelay(db_relay));
            }
            Message::ToggleWrite(mut db_relay) => {
                db_relay.write = !db_relay.write;
                back_conn.send(net::Message::UpdateRelay(db_relay));
            }
            Message::ToggleAdvertise(mut db_relay) => {
                db_relay.advertise = !db_relay.advertise;
                back_conn.send(net::Message::UpdateRelay(db_relay));
            }
            Message::Ready(mut conn) => {
                conn.send(Input::GetStatus);
            }
        }
        Command::none()
    }
    pub fn new(relay: Relay, db_relay: DbRelay) -> Result<Self, Error> {
        Ok(Self {
            status: db_relay.status,
            url: db_relay.url,
            last_connected_at: db_relay.last_connected_at,
            is_read: db_relay.read,
            is_write: db_relay.write,
            _is_advertise: false,
            relay,
        })
    }
    fn seconds_since_last_conn(&self) -> i64 {
        let now = chrono::Utc::now().timestamp_millis() / 1000;
        if let Some(last_connected_at) = self.last_connected_at {
            now - last_connected_at
        } else {
            0
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        row![
            text(&self.status).width(Length::Fill),
            container(text(&self.url)).width(Length::Fill),
            container(text(format!("{}s", self.seconds_since_last_conn()))).width(Length::Fill),
            container(checkbox("", self.is_read, |_| Message::ToggleRead(
                self.into_db_relay()
            )))
            .width(Length::Fill),
            container(checkbox("", self.is_write, |_| Message::ToggleWrite(
                self.into_db_relay()
            )))
            .width(Length::Fill),
            // container(checkbox("", self.is_advertise, |_| {
            //     Message::ToggleAdvertise
            // }))
            // .width(Length::Fill),
            button("Connect")
                .on_press(Message::ConnectToRelay(self.url.clone()))
                .width(Length::Fill),
            button("Remove")
                .on_press(Message::DeleteRelay(self.url.clone()))
                .width(Length::Fill),
        ]
        .into()
    }
    pub fn view_header() -> Element<'static, Message> {
        row![
            text("Status").width(Length::Fill),
            text("Address").width(Length::Fill),
            text("Last Active").width(Length::Fill),
            text("Read").width(Length::Fill),
            text("Write").width(Length::Fill),
            // text("Advertise").width(Length::Fill),
            text("").width(Length::Fill),
            text("").width(Length::Fill)
        ]
        .into()
    }
}

// impl From<&RelayRow> for DbRelay {
//     fn from(row: &RelayRow) -> Self {
//         Self {
//             url: row.url.to_owned(),
//             last_connected_at: row.last_connected_at,
//             read: row.is_read,
//             write: row.is_write,
//             advertise: row.is_advertise,
//         }
//     }
// }
