use crate::db::DbRelay;
use crate::icon::{delete_icon, exclamation_icon, solid_circle_icon};
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::widget::{Element, Text};
use chrono::{NaiveDateTime, Utc};
use iced::widget::{button, checkbox, container, row, text, tooltip, Space};
use iced::{alignment, Command, Length, Subscription};
use ns_client::RelayStatus;

#[derive(Debug, Clone)]
pub enum RelayRowState {
    Idle,
    Loading,
    Success,
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
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    DeleteRelay(DbRelay),
    ToggleRead(DbRelay),
    ToggleWrite(DbRelay),
    SendContactListToRelays,
    UpdateRelayStatus(RelayStatus),
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

#[derive(Debug, Clone)]
pub struct RelayRow {
    pub id: i32,
    pub db_relay: DbRelay,
    relay_status: Option<RelayStatus>,
    last_connected_at: Option<NaiveDateTime>,
    mode: Mode,
}

impl RelayRow {
    pub fn new(id: i32, db_relay: DbRelay, _conn: &mut BackEndConnection) -> Self {
        Self {
            id,
            db_relay,
            relay_status: None,
            last_connected_at: None,
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
        Subscription::none()
    }

    pub fn backend_event(&mut self, event: BackendEvent, _conn: &mut BackEndConnection) {
        match event {
            BackendEvent::RelayUpdated(db_relay) => {
                if self.db_relay.url == db_relay.url {
                    tracing::debug!("Relay updated");
                    tracing::debug!("{:?}", db_relay);
                    self.db_relay = db_relay;
                }
            }
            // BackendEvent::GotRelayStatus((url, relay_status)) => {
            //     if self.db_relay.url == url {
            //         self.relay_status = Some(relay_status)
            //     }
            // }
            BackendEvent::ConfirmedContactList(db_event) => {
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
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Command<MessageWrapper> {
        match message {
            Message::None => (),
            Message::UpdateRelayStatus(status) => {
                self.relay_status = Some(status);
            }
            Message::SendContactListToRelays => {
                if let Mode::ModalView { .. } = &mut self.mode {
                    conn.send(net::ToBackend::SendContactListToRelays);
                    self.mode.loading();
                }
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
        }
        Command::none()
    }

    fn seconds_since_last_conn(&self) -> Element<'static, MessageWrapper> {
        if let Some(last_connected_at) = self.last_connected_at {
            let now = Utc::now().naive_utc();
            let dif_secs = (now - last_connected_at).num_seconds();
            text(format!("{}s", &dif_secs)).into()
        } else {
            text("").into()
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
        let (status_icon, status_text) = match &self.relay_status {
            Some(status) => {
                let status_icon = match status {
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
                let status_text = status.to_string();
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
