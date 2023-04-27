use std::ops::Not;

use crate::error::Error;
use crate::icon::{circle_icon, delete_icon, server_icon};
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::Element;
use iced::widget::{button, checkbox, container, row, text};
use iced::{alignment, Command, Length, Subscription};
use iced_native::futures::channel::mpsc;
use nostr_sdk::{Relay, RelayStatus, Url};

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
    ConnectToRelay(Url),
    UpdateStatus((Url, RelayStatus, i64)),
    DeleteRelay(Url),
    ToggleRead((Url, bool)),
    ToggleWrite((Url, bool)),
    // ToggleAdvertise((Url, bool)),
    Ready(RelayRowConnection),
}

pub enum Input {
    GetStatus,
}
pub enum State {
    Idle {
        url: Url,
        relay: Relay,
    },
    Querying {
        url: Url,
        receiver: mpsc::UnboundedReceiver<Input>,
        relay: Relay,
    },
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    url: Url,
    status: RelayStatus,
    last_connected_at: i64,
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

                        let relay_status = match input {
                            Input::GetStatus => relay.status().await,
                        };

                        (
                            Message::UpdateStatus((
                                url.clone(),
                                relay_status,
                                relay.stats().connected_at().as_i64(),
                            )),
                            State::Idle { url, relay },
                        )
                    }
                }
            },
        )
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
                    self.last_connected_at = last_connected_at;
                    // self.last_connected_at = Some(last_connected_at);
                    // back_conn.send(net::Message::UpdateRelay(self.into_db_relay()));
                }
            }
            Message::DeleteRelay(relay_url) => {
                back_conn.send(net::Message::DeleteRelay(relay_url));
            }
            Message::ToggleRead((url, read)) => {
                back_conn.send(net::Message::ToggleRelayRead((url, read)));
            }
            Message::ToggleWrite((url, write)) => {
                back_conn.send(net::Message::ToggleRelayWrite((url, write)));
            }
            // Message::ToggleAdvertise(mut db_relay) => {
            //     // db_relay.advertise = !db_relay.advertise;
            //     back_conn.send(net::Message::UpdateRelay(db_relay));
            // }
            Message::Ready(mut conn) => {
                conn.send(Input::GetStatus);
            }
        }
        Command::none()
    }
    pub fn new(relay: Relay) -> Result<Self, Error> {
        Ok(Self {
            status: RelayStatus::Disconnected,
            url: relay.url().clone(),
            last_connected_at: relay.stats().connected_at().as_i64(),
            is_read: relay.opts().read(),
            is_write: relay.opts().write(),
            _is_advertise: false,
            relay,
        })
    }
    fn seconds_since_last_conn(&self) -> i64 {
        let now = chrono::Utc::now().timestamp_millis() / 1000;
        now - self.last_connected_at
    }
    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let status_icon = match self.status {
            RelayStatus::Initialized => circle_icon()
                .size(16)
                .style(style::Text::RelayStatusInitialized),
            RelayStatus::Connected => circle_icon()
                .size(16)
                .style(style::Text::RelayStatusConnected),
            RelayStatus::Connecting => circle_icon()
                .size(16)
                .style(style::Text::RelayStatusConnecting),
            RelayStatus::Disconnected => circle_icon()
                .size(16)
                .style(style::Text::RelayStatusDisconnected),
            RelayStatus::Terminated => circle_icon()
                .size(16)
                .style(style::Text::RelayStatusTerminated),
        };
        let activity_time = format!("{}s", self.seconds_since_last_conn());

        container(
            row![
                status_icon.width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
                container(text(&self.url)).center_x().width(Length::Fill),
                container(text(&activity_time))
                    .center_x()
                    .width(Length::Fixed(ACTIVITY_CELL_WIDTH)),
                container(checkbox("", self.is_read, |_| Message::ToggleRead((
                    self.relay.url(),
                    self.relay.opts().read().not()
                ))))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                container(checkbox("", self.is_write, |_| Message::ToggleWrite((
                    self.relay.url(),
                    self.relay.opts().write().not()
                ))))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                button(server_icon().size(16))
                    .on_press(Message::ConnectToRelay(self.url.clone()))
                    .width(Length::Fixed(ACTION_ICON_WIDTH)),
                button(delete_icon().size(16))
                    .on_press(Message::DeleteRelay(self.url.clone()))
                    .width(Length::Fixed(ACTION_ICON_WIDTH)),
            ]
            .align_items(alignment::Alignment::Center),
        )
        // queria um hover para cada linha da tabela
        // .style(style::Container::TableRow)
        .into()
    }
    pub fn view_header() -> Element<'static, Message> {
        row![
            container(text("")).width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
            container(text("Address")).center_x().width(Length::Fill),
            container(text("Activity"))
                .center_x()
                .width(Length::Fixed(ACTIVITY_CELL_WIDTH)),
            container(text("Read"))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
            container(text("Write"))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
            container(text(""))
                .center_x()
                .width(Length::Fixed(ACTION_ICON_WIDTH)),
            container(text(""))
                .center_x()
                .width(Length::Fixed(ACTION_ICON_WIDTH))
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
const RELAY_STATUS_ICON_WIDTH: f32 = 30.0;
const ACTION_ICON_WIDTH: f32 = 30.0;
const CHECKBOX_CELL_WIDTH: f32 = 50.0;
const ACTIVITY_CELL_WIDTH: f32 = 100.0;
