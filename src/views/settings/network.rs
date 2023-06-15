use std::time::Duration;

use crate::components::text::title;
use crate::components::{common_scrollable, relay_row, RelayRow};
use crate::error::BackendClosed;
use crate::icon::plus_icon;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::utils::url_matches_search;
use crate::widget::Element;
use iced::alignment::{self};
use iced::widget::{button, column, container, row, text, text_input, tooltip, Space};
use iced::{Alignment, Length, Subscription};

use super::SettingsRouterMessage;

#[derive(Debug, Clone)]
pub enum Message {
    RelayRowMessage(relay_row::MessageWrapper),
    OpenAddRelayModal,
    SearchInputChange(String),
    Tick,
    SyncWithNTP,
}

pub struct NtpInfo {
    last_ntp_offset: i64,
    ntp_offset: Option<i64>,
    ntp_server: Option<String>,
}

pub struct State {
    relays: Vec<RelayRow>,
    search_input: String,
    ntp_info: Option<NtpInfo>,
    ntp_btn_enabled: bool,
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(TICK_INTERVAL_MILLIS)).map(|_| Message::Tick)
    }
    pub fn new(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        conn.send(net::ToBackend::FetchRelays)?;
        conn.send(net::ToBackend::GetNtpInfo)?;
        Ok(Self {
            relays: vec![],
            search_input: "".into(),
            ntp_info: None,
            ntp_btn_enabled: false,
        })
    }

    pub fn backend_event(&mut self, event: BackendEvent, _conn: &mut BackEndConnection) {
        match event {
            BackendEvent::NtpInfo {
                last_ntp_offset,
                ntp_offset,
                ntp_server,
            } => {
                if ntp_server.is_none() {
                    self.ntp_btn_enabled = true;
                }

                self.ntp_info = Some(NtpInfo {
                    last_ntp_offset,
                    ntp_offset,
                    ntp_server,
                })
            }
            BackendEvent::RelayUpdated(db_relay) => {
                if let Some(row) = self
                    .relays
                    .iter_mut()
                    .find(|row| row.db_relay.url == db_relay.url)
                {
                    row.relay_updated(db_relay);
                } else {
                    tracing::warn!("Got information for unknown relay: {}", db_relay.url);
                }
            }
            BackendEvent::RelayCreated(db_relay) => self
                .relays
                .push(RelayRow::new(self.relays.len() as i32, db_relay)),
            BackendEvent::RelayDeleted(url) => {
                self.relays.retain(|r| r.db_relay.url != url);
            }
            BackendEvent::GotRelays(mut db_relays) => {
                db_relays.sort_by(|a, b| a.url.cmp(&b.url));
                self.relays = db_relays
                    .into_iter()
                    .enumerate()
                    .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay))
                    .collect();
            }
            _ => (),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<Option<SettingsRouterMessage>, BackendClosed> {
        match message {
            Message::Tick => {
                if !self.relays.is_empty() {
                    conn.send(net::ToBackend::GetRelayInformation)?;
                }
            }
            Message::SearchInputChange(text) => {
                self.search_input = text;
            }
            Message::OpenAddRelayModal => {
                return Ok(Some(SettingsRouterMessage::OpenRelayBasicModal));
            }

            Message::RelayRowMessage(msg) => match msg.message {
                relay_row::Message::OpenRelayDocument(db_relay) => {
                    return Ok(Some(SettingsRouterMessage::OpenRelayDocument(db_relay)));
                }
                other => {
                    if let Some(row) = self.relays.iter_mut().find(|r| r.id == msg.from) {
                        let _ = row.update(other, conn)?;
                    }
                }
            },
            Message::SyncWithNTP => {
                self.ntp_btn_enabled = false;
                conn.send(net::ToBackend::SyncWithNTP)?;
            }
        }

        Ok(None)
    }

    pub fn view(&self) -> Element<Message> {
        let page_title = title("Network").height(HEADER_HEIGHT);
        let ntp_title = text("NTP Server").size(24);

        let ntp_content: Element<_> = if let Some(info) = &self.ntp_info {
            let synced_with: Element<_> = if let Some(server) = &info.ntp_server {
                let server_input = text_input("", &server).style(style::TextInput::ChatSearch);
                row![text("Synced with NTP server").width(200), server_input,]
                    .align_items(Alignment::Center)
                    .spacing(5)
                    .into()
            } else {
                text("Not synced with NTP server").into()
            };

            let last_offset_input = text_input("", &info.last_ntp_offset.to_string())
                .style(style::TextInput::ChatSearch);
            let mut sync_btn = button("Sync").style(style::Button::Primary);
            if self.ntp_btn_enabled {
                sync_btn = sync_btn.on_press(Message::SyncWithNTP);
            }

            column![
                synced_with,
                row![text("Time Offset").width(200), last_offset_input,]
                    .align_items(Alignment::Center)
                    .spacing(5),
                row![Space::with_width(Length::Fill), sync_btn]
            ]
            .spacing(5)
            .into()
        } else {
            text("Loading...").into()
        };
        let ntp_gp = column![ntp_title, ntp_content,].spacing(10);

        let relays_title = text("Relays").size(24);

        let add_btn = tooltip(
            button(
                row![text("Add").size(18), plus_icon().size(14)]
                    .align_items(alignment::Alignment::Center)
                    .spacing(2),
            )
            .padding(5)
            .on_press(Message::OpenAddRelayModal),
            "Add Relay",
            tooltip::Position::Top,
        )
        .style(style::Container::TooltipBg);
        let search_input = text_input("Search", &self.search_input)
            .on_input(Message::SearchInputChange)
            .style(style::TextInput::ChatSearch)
            .width(SEARCH_WIDTH);
        let utils_row = row![search_input, Space::with_width(Length::Fill), add_btn];

        let table_header = column![RelayRow::view_header().map(|mut message| {
            message.from = -1;
            Message::RelayRowMessage(message)
        })];
        let relay_rows = self
            .relays
            .iter()
            .filter(|row| url_matches_search(&row.db_relay.url, &self.search_input))
            .fold(column![].spacing(4), |col, relay| {
                col.push(relay.view().map(Message::RelayRowMessage))
            });
        let relays_table = container(table_header.push(relay_rows));
        let relays_gp = column![relays_title, utils_row, relays_table].spacing(5);

        container(common_scrollable(
            column![page_title, ntp_gp, relays_gp]
                .spacing(10)
                .padding([0, 20, 0, 0]),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

const HEADER_HEIGHT: f32 = 50.0;
const SEARCH_WIDTH: f32 = 200.0;
const TICK_INTERVAL_MILLIS: u64 = 500;
