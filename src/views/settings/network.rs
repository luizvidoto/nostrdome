use std::time::Duration;

use iced::alignment::{self, Horizontal};
use iced::widget::{button, column, container, row, text, text_input, tooltip, Space};
use iced::{Length, Subscription};
use iced_aw::{Card, Modal};
use nostr::Url;

use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::components::{common_scrollable, relay_row, RelayRow};
use crate::icon::plus_icon;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::utils::url_matches_search;
use crate::widget::Element;

use super::SettingsRouterMessage;

#[derive(Debug, Clone)]
pub enum Message {
    RelayRowMessage(relay_row::MessageWrapper),
    OpenAddRelayModal,
    CancelButtonPressed,
    OkButtonPressed,
    CloseModal,
    AddRelayInputChange(String),
    SearchInputChange(String),
    Tick,
}

pub struct State {
    relays: Vec<RelayRow>,
    show_modal: bool,
    add_relay_input: String,
    is_invalid: bool,
    search_input: String,
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(TICK_INTERVAL_MILLIS)).map(|_| Message::Tick)
    }
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchRelays);
        Self {
            relays: vec![],
            show_modal: false,
            add_relay_input: "".into(),
            is_invalid: false,
            search_input: "".into(),
        }
    }

    pub fn backend_event(&mut self, event: BackendEvent, conn: &mut BackEndConnection) {
        match event {
            BackendEvent::RelayUpdated(db_relay) => {
                if let Some(row) = self
                    .relays
                    .iter_mut()
                    .find(|row| row.db_relay.url == db_relay.url)
                {
                    let _ = row.update(relay_row::Message::RelayUpdated(db_relay), conn);
                } else {
                    tracing::warn!("Got information for unknown relay: {}", db_relay.url);
                }
            }
            BackendEvent::RelayCreated(db_relay) => {
                // conn.send(net::ToBackend::RequestEventsOf(db_relay.clone()));
                self.relays
                    .push(RelayRow::new(self.relays.len() as i32, db_relay, conn))
            }
            BackendEvent::RelayDeleted(url) => {
                self.relays.retain(|r| r.db_relay.url != url);
            }
            BackendEvent::GotRelays(mut db_relays) => {
                db_relays.sort_by(|a, b| a.url.cmp(&b.url));
                self.relays = db_relays
                    .into_iter()
                    .enumerate()
                    .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay, conn))
                    .collect();
            }
            _ => (),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Option<SettingsRouterMessage> {
        match message {
            Message::Tick => {
                if !self.relays.is_empty() {
                    conn.send(net::ToBackend::GetRelayInformation);
                }
            }
            Message::SearchInputChange(text) => {
                self.search_input = text;
            }
            Message::AddRelayInputChange(relay_addrs) => {
                self.add_relay_input = relay_addrs;
                self.is_invalid = false;
            }
            Message::CloseModal | Message::CancelButtonPressed => {
                self.add_relay_input = "".into();
                self.show_modal = false;
            }
            Message::OkButtonPressed => match Url::try_from(self.add_relay_input.as_str()) {
                Ok(url) => {
                    self.is_invalid = false;
                    self.show_modal = false;
                    self.add_relay_input = "".into();
                    conn.send(net::ToBackend::AddRelay(url));
                }
                Err(e) => {
                    tracing::error!("{}", e);
                    self.is_invalid = true;
                }
            },
            Message::OpenAddRelayModal => self.show_modal = true,

            Message::RelayRowMessage(msg) => match msg.message {
                relay_row::Message::OpenRelayDocument(db_relay) => {
                    return Some(SettingsRouterMessage::OpenRelayDocument(db_relay));
                }
                other => {
                    if let Some(row) = self.relays.iter_mut().find(|r| r.id == msg.from) {
                        let _ = row.update(other, conn);
                    }
                }
            },
        }

        None
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Network").height(HEADER_HEIGHT);

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
        let relays_ct = container(table_header.push(common_scrollable(relay_rows)));

        let content: Element<_> = container(column![title, utils_row, relays_ct])
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        Modal::new(self.show_modal, content, || {
            let mut add_relay_input = TextInputGroup::new(
                "Relay Address",
                &self.add_relay_input,
                Message::AddRelayInputChange,
            )
            .placeholder("wss://my-relay.com")
            .on_submit(Message::OkButtonPressed);

            if self.is_invalid {
                add_relay_input = add_relay_input.invalid("Relay address is invalid");
            }

            let modal_body: Element<_> = container(add_relay_input.build()).into();
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
                .max_width(CARD_MAX_WIDTH)
                .on_close(Message::CloseModal)
                .into()
        })
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal)
        .into()
    }
}

const CARD_MAX_WIDTH: f32 = 300.0;
const HEADER_HEIGHT: f32 = 50.0;
const SEARCH_WIDTH: f32 = 200.0;
const TICK_INTERVAL_MILLIS: u64 = 500;
