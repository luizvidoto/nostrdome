use iced::alignment::{self, Horizontal};
use iced::widget::{button, column, container, row, text, tooltip};
use iced::{Command, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr_sdk::Url;

use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::components::{relay_row, RelayRow};
use crate::db::DbRelay;
use crate::icon::plus_icon;
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::MessageWrapper),
    BackEndEvent(Event),
    OpenAddRelayModal,
    CancelButtonPressed,
    OkButtonPressed,
    CloseModal,
    AddRelayInputChange(String),
}

#[derive(Debug, Clone)]
pub struct State {
    relays: Vec<RelayRow>,
    show_modal: bool,
    add_relay_input: String,
    is_invalid: bool,
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
        conn.send(net::Message::FetchRelays);
        Self {
            relays: vec![],
            show_modal: false,
            add_relay_input: "".into(),
            is_invalid: false,
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        match message {
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
                    conn.send(net::Message::AddRelay(db_relay));
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
                Event::RelayCreated(db_relay) => {
                    conn.send(net::Message::RequestEventsOf(db_relay.clone()));
                    self.relays
                        .push(RelayRow::new(self.relays.len() as i32, db_relay, conn))
                }
                Event::RelayDeleted(db_relay) => {
                    self.relays.retain(|r| r.db_relay.url != db_relay.url);
                }
                Event::GotRelays(mut db_relays) => {
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
        let title = title("Network");
        let header = column![RelayRow::view_header().map(|mut message| {
            message.from = -1;
            Message::RelayMessage(message)
        })];
        let relays = self.relays.iter().fold(header, |col, relay| {
            col.push(relay.view().map(Message::RelayMessage))
        });
        let empty = container(text("")).width(Length::Fill);
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
        let add_row = row![empty, add_btn];
        let content: Element<_> = container(column![title, add_row, relays])
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
