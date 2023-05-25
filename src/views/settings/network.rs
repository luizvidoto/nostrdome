use iced::alignment::{self, Horizontal};
use iced::widget::{button, column, container, row, text, text_input, tooltip, Space};
use iced::{Command, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr::Url;

use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::components::{common_scrollable, relay_row, RelayRow};
use crate::db::DbRelay;
use crate::icon::plus_icon;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::utils::url_matches_search;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::MessageWrapper),
    BackEndEvent(BackendEvent),
    OpenAddRelayModal,
    CancelButtonPressed,
    OkButtonPressed,
    CloseModal,
    AddRelayInputChange(String),
    SearchInputChange(String),
}

#[derive(Debug, Clone)]
pub struct State {
    relays: Vec<RelayRow>,
    show_modal: bool,
    add_relay_input: String,
    is_invalid: bool,
    search_input: String,
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        let relay_subs: Vec<_> = self
            .relays
            .iter()
            .map(|r| r.subscription().map(Message::RelayMessage))
            .collect();
        iced::Subscription::batch(relay_subs)
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

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        match message {
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
                    let db_relay = DbRelay::new(url);
                    conn.send(net::ToBackend::AddRelay(db_relay));
                }
                Err(e) => {
                    tracing::error!("{}", e);
                    self.is_invalid = true;
                }
            },
            Message::OpenAddRelayModal => self.show_modal = true,

            Message::RelayMessage(msg) => {
                if let Some(row) = self.relays.iter_mut().find(|r| r.id == msg.from) {
                    let _ = row.update(msg, conn);
                }
            }

            Message::BackEndEvent(ev) => match ev {
                BackendEvent::RelayCreated(db_relay) => {
                    conn.send(net::ToBackend::RequestEventsOf(db_relay.clone()));
                    self.relays
                        .push(RelayRow::new(self.relays.len() as i32, db_relay, conn))
                }
                BackendEvent::RelayDeleted(db_relay) => {
                    self.relays.retain(|r| r.db_relay.url != db_relay.url);
                }
                BackendEvent::GotRelays(mut db_relays) => {
                    db_relays.sort_by(|a, b| a.url.cmp(&b.url));
                    self.relays = db_relays
                        .into_iter()
                        .enumerate()
                        .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay, conn))
                        .collect();
                }
                other => {
                    self.relays
                        .iter_mut()
                        .for_each(|r| r.backend_event(other.clone(), conn));
                }
            },
        }
        Command::none()
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
            Message::RelayMessage(message)
        })];
        let relay_rows = self
            .relays
            .iter()
            .filter(|row| url_matches_search(&row.db_relay.url, &self.search_input))
            .fold(column![].spacing(4), |col, relay| {
                col.push(relay.view().map(Message::RelayMessage))
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
