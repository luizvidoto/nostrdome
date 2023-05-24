use crate::db::DbRelay;
use crate::icon::{delete_icon, exclamation_icon, solid_circle_icon};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::utils::ns_event_to_naive;
use crate::widget::{Element, Text};
use chrono::Utc;
use iced::futures::channel::mpsc;
use iced::widget::{button, checkbox, container, row, text, tooltip, Space};
use iced::{alignment, Command, Length, Subscription};
use nostr::Timestamp;
use nostr_sdk::{Relay, RelayStatus};

#[derive(Debug, Clone)]
pub struct RelayRowConnection(mpsc::Sender<Input>);
impl RelayRowConnection {
    pub fn send(&mut self, input: Input) {
        if let Err(e) = self.0.try_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}

#[derive(Debug, Clone)]
pub enum RelayRowState {
    Idle,
    Loading,
    Success,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Mode {
    ModalView { state: RelayRowState },
    Normal,
}
impl Mode {
    pub fn success(&mut self) {
        if let Mode::ModalView { state } = self {
            *state = RelayRowState::Success;
        }
    }
    pub fn loading(&mut self) {
        if let Mode::ModalView { state } = self {
            *state = RelayRowState::Loading;
        }
    }
    pub fn error(&mut self, error: String) {
        if let Mode::ModalView { state } = self {
            *state = RelayRowState::Error(error);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConnectToRelay(DbRelay),
    UpdateStatus((RelayStatus, Timestamp)),
    DeleteRelay(DbRelay),
    ToggleRead(DbRelay),
    ToggleWrite(DbRelay),
    Ready(RelayRowConnection),
    Performing,
    Waited,
    SendContactListToRelays,
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
    Start {
        id: i32,
    },
    Idle {
        id: i32,
        receiver: mpsc::Receiver<Input>,
    },
    Performing {
        id: i32,
        receiver: mpsc::Receiver<Input>,
        channel_relay: Relay,
    },
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    pub id: i32,
    pub db_relay: DbRelay,
    client_relay: Option<Relay>,
    sub_channel: Option<RelayRowConnection>,
    mode: Mode,
}

impl RelayRow {
    pub fn new(id: i32, db_relay: DbRelay, conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchRelayServer(db_relay.url.clone()));
        Self {
            id,
            db_relay,
            client_relay: None,
            sub_channel: None,
            mode: Mode::Normal,
        }
    }
    pub fn with_mode(mut self) -> Self {
        self.mode = Mode::ModalView {
            state: RelayRowState::Idle,
        };
        self
    }
    pub fn subscription(&self) -> Subscription<MessageWrapper> {
        // let unique_id = uuid::Uuid::new_v4().to_string();
        iced::subscription::unfold(
            self.db_relay.url.clone(),
            State::Start { id: self.id },
            |state| async move {
                match state {
                    State::Start { id } => {
                        let (sender, receiver) = mpsc::channel(1024);
                        (
                            MessageWrapper::new(id, Message::Ready(RelayRowConnection(sender))),
                            State::Idle { receiver, id },
                        )
                    }
                    State::Idle { mut receiver, id } => {
                        use iced::futures::StreamExt;

                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        let input = receiver.select_next_some().await;

                        match input {
                            Input::GetStatus(channel_relay) => (
                                MessageWrapper::new(id, Message::Performing),
                                State::Performing {
                                    id,
                                    receiver,
                                    channel_relay,
                                },
                            ),
                            Input::Wait => {
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                (
                                    MessageWrapper::new(id, Message::Waited),
                                    State::Idle { receiver, id },
                                )
                            }
                        }
                    }
                    State::Performing {
                        id,
                        channel_relay,
                        receiver,
                    } => {
                        let relay_status = channel_relay.status().await;
                        if let RelayStatus::Initialized = &relay_status {
                            channel_relay.connect(true).await
                        }
                        (
                            MessageWrapper::new(
                                id,
                                Message::UpdateStatus((
                                    relay_status,
                                    channel_relay.stats().connected_at(),
                                )),
                            ),
                            State::Idle { receiver, id },
                        )
                    }
                }
            },
        )
    }

    pub fn backend_event(&mut self, event: Event, _conn: &mut BackEndConnection) {
        match event {
            Event::RelayUpdated(db_relay) => {
                if self.db_relay.url == db_relay.url {
                    tracing::debug!("Relay updated");
                    tracing::debug!("{:?}", db_relay);
                    self.db_relay = db_relay;
                }
            }
            Event::GotRelayServer(relay) => {
                if let Some(relay) = relay {
                    if self.db_relay.url == relay.url() {
                        self.client_relay = Some(relay);
                    }
                }
            }
            Event::ConfirmedContactList(db_event) => {
                if let Some(relay_url) = db_event.relay_url {
                    if relay_url == self.db_relay.url {
                        self.mode.success()
                    }
                }
            }
            _ => (),
        }
    }

    pub fn update(
        &mut self,
        wrapper: MessageWrapper,
        conn: &mut BackEndConnection,
    ) -> Command<MessageWrapper> {
        match wrapper.message {
            Message::None => (),
            Message::SendContactListToRelays => {
                if let Mode::ModalView { .. } = &mut self.mode {
                    conn.send(net::ToBackend::SendContactListToRelays);
                    self.mode.loading();
                }
            }

            Message::ConnectToRelay(db_relay) => {
                conn.send(net::ToBackend::ConnectToRelay(db_relay));
            }
            Message::DeleteRelay(db_relay) => {
                conn.send(net::ToBackend::DeleteRelay(db_relay));
            }
            Message::ToggleRead(db_relay) => {
                let read = !db_relay.read;
                conn.send(net::ToBackend::ToggleRelayRead((db_relay, read)));
            }
            Message::ToggleWrite(db_relay) => {
                let write = !db_relay.write;
                conn.send(net::ToBackend::ToggleRelayWrite((db_relay, write)));
            }
            Message::UpdateStatus((status, last_connected_at)) => {
                self.db_relay = self.db_relay.clone().with_status(status);
                if last_connected_at.as_i64() != 0 {
                    if let Ok(last_connected_at) = ns_event_to_naive(last_connected_at) {
                        self.db_relay = self
                            .db_relay
                            .clone()
                            .with_last_connected_at(last_connected_at);
                    }
                } else {
                    self.db_relay.last_connected_at = None;
                }

                if let (Some(ch), Some(relay)) = (&mut self.sub_channel, &self.client_relay) {
                    ch.send(Input::GetStatus(relay.clone()));
                }
            }
            Message::Performing => {
                tracing::debug!("Relay Row performing");
            }
            Message::Waited => {
                tracing::debug!("Message::Waited");
                self.send_action_to_channel(conn);
            }
            Message::Ready(channel) => {
                tracing::debug!("Message::Ready(channel)");
                self.sub_channel = Some(channel);
                self.send_action_to_channel(conn);
            }
        }
        Command::none()
    }

    fn send_action_to_channel(&mut self, conn: &mut BackEndConnection) {
        if let Some(ch) = &mut self.sub_channel {
            match &self.client_relay {
                Some(relay) => {
                    ch.send(Input::GetStatus(relay.clone()));
                }
                None => {
                    // fetch relays again?
                    conn.send(net::ToBackend::FetchRelayServer(self.db_relay.url.clone()));
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
    fn is_connected(&self) -> bool {
        match &self.db_relay.status {
            Some(last_active) => match last_active.0 {
                RelayStatus::Connected => true,
                _ => false,
            },
            None => false,
        }
    }

    pub fn view_header() -> Element<'static, MessageWrapper> {
        row![
            container(text("")).width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
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
                .width(Length::Fixed(ACTION_ICON_WIDTH))
        ]
        .into()
    }

    pub fn view<'a>(&'a self) -> Element<'a, MessageWrapper> {
        let (status_icon, status_text) = self.relay_status_icon();
        let delete_btn = tooltip(
            button(delete_icon().size(16))
                .on_press(MessageWrapper::new(
                    self.id,
                    Message::DeleteRelay(self.db_relay.clone()),
                ))
                .style(style::Button::Danger)
                .width(Length::Fixed(ACTION_ICON_WIDTH)),
            "Delete Relay",
            tooltip::Position::Left,
        )
        .style(style::Container::TooltipBg);

        container(
            row![
                tooltip(
                    status_icon.width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
                    status_text,
                    tooltip::Position::Top
                )
                .style(style::Container::TooltipBg),
                self.have_error_icon(),
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
                delete_btn,
            ]
            .align_items(alignment::Alignment::Center),
        )
        // queria um hover para cada linha da tabela
        // .style(style::Container::TableRow)
        .into()
    }

    fn have_error_icon<'a, M: 'a>(&self) -> Element<'a, M> {
        if let Some(error_msg) = &self.db_relay.have_error {
            tooltip(
                exclamation_icon().size(16).style(style::Text::Danger),
                error_msg,
                tooltip::Position::Top,
            )
            .style(style::Container::TooltipBg)
            .into()
        } else {
            text("").into()
        }
    }

    fn relay_status_icon<'a>(&'a self) -> (Text<'a>, String) {
        let (status_icon, status_text) = match &self.db_relay.status {
            Some(last_active) => {
                let status_icon = match last_active.0 {
                    RelayStatus::Initialized => solid_circle_icon()
                        .size(16)
                        .style(style::Text::RelayStatusInitialized),
                    RelayStatus::Connected => solid_circle_icon()
                        .size(16)
                        .style(style::Text::RelayStatusConnected),
                    RelayStatus::Connecting => solid_circle_icon()
                        .size(16)
                        .style(style::Text::RelayStatusConnecting),
                    RelayStatus::Disconnected => solid_circle_icon()
                        .size(16)
                        .style(style::Text::RelayStatusDisconnected),
                    RelayStatus::Terminated => solid_circle_icon()
                        .size(16)
                        .style(style::Text::RelayStatusTerminated),
                };
                let status_text = last_active.0.to_string();
                (status_icon, status_text)
            }
            None => (
                solid_circle_icon()
                    .size(16)
                    .style(style::Text::RelayStatusLoading),
                "Loading".into(),
            ),
        };
        (status_icon, status_text)
    }

    pub fn modal_view(&self) -> Element<MessageWrapper> {
        if let Mode::ModalView { state } = &self.mode {
            let button_or_checkmark: Element<_> = match state {
                RelayRowState::Idle => {
                    let mut btn = button("Send").style(style::Button::Primary);
                    if self.is_connected() {
                        btn = btn.on_press(MessageWrapper::new(
                            self.id,
                            Message::SendContactListToRelays,
                        ))
                    }
                    btn.into()
                }

                RelayRowState::Loading => button("...").style(style::Button::Primary).into(),
                RelayRowState::Success => text("Sent!").into(),
                RelayRowState::Error(_) => text("Error").into(),
            };

            let (status_icon, status_text) = self.relay_status_icon();

            container(
                row![
                    tooltip(
                        status_icon.width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
                        status_text,
                        tooltip::Position::Top
                    )
                    .style(style::Container::TooltipBg),
                    text(&self.db_relay.url),
                    Space::with_width(Length::Fill),
                    button_or_checkmark
                ]
                .align_items(alignment::Alignment::Center),
            )
            .center_y()
            .into()
        } else {
            text("").into()
        }
    }
    pub fn relay_welcome(&self) -> Element<MessageWrapper> {
        let (status_icon, status_text) = self.relay_status_icon();
        container(
            row![
                tooltip(
                    status_icon.width(Length::Fixed(RELAY_STATUS_ICON_WIDTH)),
                    status_text,
                    tooltip::Position::Top
                )
                .style(style::Container::TooltipBg),
                text(&self.db_relay.url),
                Space::with_width(Length::Fill),
                tooltip(
                    button(delete_icon())
                        .on_press(MessageWrapper::new(
                            self.id,
                            Message::DeleteRelay(self.db_relay.clone())
                        ))
                        .style(style::Button::Danger),
                    "Delete",
                    tooltip::Position::Top
                )
                .style(style::Container::TooltipBg)
            ]
            .align_items(alignment::Alignment::Center),
        )
        .center_y()
        .into()
    }
}

const RELAY_STATUS_ICON_WIDTH: f32 = 30.0;
const ACTION_ICON_WIDTH: f32 = 30.0;
const CHECKBOX_CELL_WIDTH: f32 = 50.0;
const ACTIVITY_CELL_WIDTH: f32 = 100.0;
