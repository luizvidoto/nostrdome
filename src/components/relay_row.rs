use crate::db::DbRelay;
use crate::icon::{circle_icon, delete_icon, server_icon};
use crate::net::{database, nostr_client, BackEndConnection, Connection};
use crate::style;
use crate::widget::Element;
use chrono::Utc;
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
    RelayUpdated(DbRelay),
    ConnectToRelay(DbRelay),
    UpdateStatus((Url, RelayStatus, i64)),
    DeleteRelay(DbRelay),
    ToggleRead(DbRelay),
    ToggleWrite(DbRelay),
    Ready(RelayRowConnection),
    GotRelay(Relay),
    Performing,
    Waited,
}
#[derive(Debug, Clone)]
pub struct MessageWrapper {
    pub from: i32,
    pub message: Message,
}
impl MessageWrapper {
    pub fn new(from: i32, message: Message) -> Self {
        Self { from, message }
    }
}

pub enum Input {
    GetStatus(Relay),
    Wait,
}
pub enum State {
    Initial {
        url: Url,
        id: i32,
    },
    Idle {
        url: Url,
        id: i32,
        receiver: mpsc::UnboundedReceiver<Input>,
    },
    Querying {
        url: Url,
        id: i32,
        receiver: mpsc::UnboundedReceiver<Input>,
        channel_relay: Relay,
    },
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    pub id: i32,
    pub db_relay: DbRelay,
    client_relay: Option<Relay>,
    sub_channel: Option<RelayRowConnection>,
}

impl RelayRow {
    pub fn new(
        id: i32,
        db_relay: DbRelay,
        ns_conn: &mut BackEndConnection<nostr_client::Message>,
    ) -> Self {
        ns_conn.send(nostr_client::Message::FetchRelay(db_relay.url.clone()));
        Self {
            id,
            db_relay,
            client_relay: None,
            sub_channel: None,
        }
    }
    pub fn subscription(&self) -> Subscription<MessageWrapper> {
        iced::subscription::unfold(
            self.db_relay.url.clone(),
            State::Initial {
                url: self.db_relay.url.clone(),
                id: self.id,
            },
            |state| async move {
                match state {
                    State::Initial { url, id } => {
                        let (sender, receiver) = mpsc::unbounded();
                        (
                            MessageWrapper::new(id, Message::Ready(RelayRowConnection(sender))),
                            State::Idle { url, receiver, id },
                        )
                    }
                    State::Idle {
                        url,
                        mut receiver,
                        id,
                    } => {
                        use iced_native::futures::StreamExt;

                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let input = receiver.select_next_some().await;

                        match input {
                            Input::GetStatus(channel_relay) => (
                                MessageWrapper::new(id, Message::Performing),
                                State::Querying {
                                    id,
                                    url,
                                    receiver,
                                    channel_relay,
                                },
                            ),
                            Input::Wait => {
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                (
                                    MessageWrapper::new(id, Message::Waited),
                                    State::Idle { url, receiver, id },
                                )
                            }
                        }
                    }
                    State::Querying {
                        id,
                        url,
                        channel_relay,
                        receiver,
                    } => {
                        let relay_status = channel_relay.status().await;
                        let last_connected_at = channel_relay.stats().connected_at().as_i64();
                        (
                            MessageWrapper::new(
                                id,
                                Message::UpdateStatus((
                                    url.clone(),
                                    relay_status,
                                    last_connected_at,
                                )),
                            ),
                            State::Idle { url, receiver, id },
                        )
                    }
                }
            },
        )
    }

    fn handle_update_status(&mut self, url: Url, _status: RelayStatus, _last_connected_at: i64) {
        println!("Relay Row UpdateStatus");
        println!("{url}");
    }

    pub fn update(
        &mut self,
        wrapper: MessageWrapper,
        db_conn: &mut BackEndConnection<database::Message>,
        ns_conn: &mut BackEndConnection<nostr_client::Message>,
    ) -> Command<MessageWrapper> {
        match wrapper.message {
            Message::RelayUpdated(db_relay) => {
                self.db_relay = db_relay;
            }
            Message::None => (),
            Message::GotRelay(relay) => {
                self.client_relay = Some(relay);
            }
            Message::ConnectToRelay(db_relay) => {
                ns_conn.send(nostr_client::Message::ConnectToRelay(db_relay.url.clone()));
            }
            Message::UpdateStatus((url, status, last_connected_at)) => {
                self.handle_update_status(url, status, last_connected_at)
            }
            Message::DeleteRelay(db_relay) => {
                db_conn.send(database::Message::DeleteRelay(db_relay));
            }
            Message::ToggleRead(db_relay) => {
                db_conn.send(database::Message::ToggleRelayRead(db_relay));
            }
            Message::ToggleWrite(db_relay) => {
                db_conn.send(database::Message::ToggleRelayWrite(db_relay));
            }
            Message::Performing => {
                tracing::info!("Relay Row performing");
            }
            Message::Waited => {
                tracing::info!("Message::Waited");
                self.send_action_to_channel();
            }
            Message::Ready(channel) => {
                tracing::info!("Message::Ready(channel)");
                self.sub_channel = Some(channel);
                self.send_action_to_channel();
            }
        }
        Command::none()
    }

    fn send_action_to_channel(&mut self) {
        if let Some(ch) = &mut self.sub_channel {
            match &self.client_relay {
                Some(relay) => {
                    ch.send(Input::GetStatus(relay.clone()));
                }
                None => {
                    ch.send(Input::Wait);
                }
            }
        }
    }

    fn seconds_since_last_conn(&self) -> Element<'static, MessageWrapper> {
        if let Some(last_connected_at) = self.db_relay.last_connected_at {
            let now = Utc::now().naive_utc();
            let dif_secs = (now - last_connected_at).num_seconds();
            text(format!("{}s", &dif_secs)).into()
        } else {
            text("").into()
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, MessageWrapper> {
        let status_icon = match self.db_relay.status.0 {
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

        container(
            row![
                status_icon.width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
                container(text(&self.db_relay.url))
                    .center_x()
                    .width(Length::Fill),
                container(self.seconds_since_last_conn())
                    .center_x()
                    .width(Length::Fixed(ACTIVITY_CELL_WIDTH)),
                container(checkbox("", self.db_relay.read, |_| MessageWrapper::new(
                    self.id,
                    Message::ToggleRead(self.db_relay.clone())
                )))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                container(checkbox("", self.db_relay.write, |_| MessageWrapper::new(
                    self.id,
                    Message::ToggleWrite(self.db_relay.clone())
                )))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                button(server_icon().size(16))
                    .on_press(MessageWrapper::new(
                        self.id,
                        Message::ConnectToRelay(self.db_relay.clone())
                    ))
                    .width(Length::Fixed(ACTION_ICON_WIDTH)),
                button(delete_icon().size(16))
                    .on_press(MessageWrapper::new(
                        self.id,
                        Message::DeleteRelay(self.db_relay.clone())
                    ))
                    .width(Length::Fixed(ACTION_ICON_WIDTH)),
            ]
            .align_items(alignment::Alignment::Center),
        )
        // queria um hover para cada linha da tabela
        // .style(style::Container::TableRow)
        .into()
    }
    pub fn view_header() -> Element<'static, MessageWrapper> {
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

const RELAY_STATUS_ICON_WIDTH: f32 = 30.0;
const ACTION_ICON_WIDTH: f32 = 30.0;
const CHECKBOX_CELL_WIDTH: f32 = 50.0;
const ACTIVITY_CELL_WIDTH: f32 = 100.0;
