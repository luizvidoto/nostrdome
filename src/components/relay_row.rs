use crate::db::DbRelay;
use crate::icon::{
    delete_icon, exclamation_icon, file_icon_regular, refresh_icon, solid_circle_icon,
};
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::{Element, Text};
use chrono::{NaiveDateTime, Utc};
use iced::widget::{button, checkbox, container, row, text, tooltip, Space};
use iced::{alignment, Command, Length};
use ns_client::RelayStatus;

#[derive(Debug, Clone)]
pub enum Message {
    DeleteRelay,
    ToggleRead,
    ToggleWrite,
    OpenRelayDocument(DbRelay),
    RelayUpdated(DbRelay),
    ReconnectRelay,
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

pub struct RelayRow {
    pub id: i32,
    pub db_relay: DbRelay,
}

impl RelayRow {
    pub fn new(id: i32, db_relay: DbRelay, _conn: &mut BackEndConnection) -> Self {
        Self { id, db_relay }
    }

    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Command<MessageWrapper> {
        match message {
            Message::OpenRelayDocument(_db_relay) => (),
            Message::RelayUpdated(db_relay) => {
                self.db_relay = db_relay;
            }
            Message::ReconnectRelay => {
                conn.send(net::ToBackend::ReconnectRelay(self.db_relay.url.to_owned()));
            }
            Message::DeleteRelay => {
                conn.send(net::ToBackend::DeleteRelay(self.db_relay.url.to_owned()));
            }
            Message::ToggleRead => {
                conn.send(net::ToBackend::ToggleRelayRead(self.db_relay.to_owned()));
            }
            Message::ToggleWrite => {
                conn.send(net::ToBackend::ToggleRelayWrite(self.db_relay.to_owned()));
            }
        }
        Command::none()
    }

    fn seconds_since_last_conn(&self) -> Element<'static, MessageWrapper> {
        if let Some(information) = &self.db_relay.information {
            if let RelayStatus::Connected = information.status {
                if let Some(last_connected_at) = NaiveDateTime::from_timestamp_millis(
                    information.conn_stats.connected_at() as i64,
                ) {
                    let now = Utc::now().naive_utc();
                    let dif_secs = (now - last_connected_at).num_seconds();
                    return text(format!("{}s", &dif_secs)).into();
                }
            }
        }

        text("").into()
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
                .width(Length::Fixed(ACTION_ICON_WIDTH)),
            container(text(""))
                .center_x()
                .width(Length::Fixed(ACTION_ICON_WIDTH)),
            container(text(""))
                .center_x()
                .width(Length::Fixed(ACTION_ICON_WIDTH))
        ]
        .into()
    }

    pub fn view<'a>(&'a self) -> Element<'a, MessageWrapper> {
        let (status_icon, status_text) = self.relay_status_icon();

        let mut doc_btn = button(file_icon_regular().size(16))
            .style(style::Button::Primary)
            .width(Length::Fixed(ACTION_ICON_WIDTH));

        if let Some(information) = &self.db_relay.information {
            if information.document.is_some() {
                doc_btn = doc_btn.on_press(MessageWrapper::new(
                    self.id,
                    Message::OpenRelayDocument(self.db_relay.clone()),
                ));
            }
        }

        let document_btn = tooltip(doc_btn, "Relay Document", tooltip::Position::Left)
            .style(style::Container::TooltipBg);

        let mut reconnect_btn = button(refresh_icon().size(16))
            .style(style::Button::Primary)
            .width(Length::Fixed(ACTION_ICON_WIDTH));
        if let Some(information) = &self.db_relay.information {
            match information.status {
                RelayStatus::Connected => (),
                _ => {
                    reconnect_btn = reconnect_btn
                        .on_press(MessageWrapper::new(self.id, Message::ReconnectRelay));
                }
            }
        }
        let reconnect_btn = tooltip(reconnect_btn, "Reconnect", tooltip::Position::Left)
            .style(style::Container::TooltipBg);

        let delete_btn = tooltip(
            button(delete_icon().size(16))
                .on_press(MessageWrapper::new(self.id, Message::DeleteRelay))
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
                    Message::ToggleRead
                )))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                container(checkbox("", self.db_relay.write, |_| MessageWrapper::new(
                    self.id,
                    Message::ToggleWrite
                )))
                .center_x()
                .width(Length::Fixed(CHECKBOX_CELL_WIDTH)),
                document_btn,
                reconnect_btn,
                delete_btn,
            ]
            .align_items(alignment::Alignment::Center),
        )
        // queria um hover para cada linha da tabela
        // .style(style::Container::TableRow)
        .into()
    }

    fn have_error_icon<'a, M: 'a>(&self) -> Element<'a, M> {
        if let Some(information) = &self.db_relay.information {
            if let Some(last_error_msg) = information.error_messages.back() {
                return tooltip(
                    exclamation_icon().size(16).style(style::Text::Danger),
                    &last_error_msg.message,
                    tooltip::Position::Top,
                )
                .style(style::Container::TooltipBg)
                .into();
            }
        }

        text("").into()
    }

    fn relay_status_icon<'a>(&'a self) -> (Text<'a>, String) {
        if let Some(information) = &self.db_relay.information {
            let status_icon = match information.status {
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
            return (status_icon, information.status.to_string());
        }

        (
            solid_circle_icon()
                .size(16)
                .style(style::Text::RelayStatusLoading),
            "Loading".into(),
        )
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
                        .on_press(MessageWrapper::new(self.id, Message::DeleteRelay))
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
