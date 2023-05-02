use iced::alignment::{self, Horizontal};
use iced::widget::{button, column, container, row, text};
use iced::{Command, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr_sdk::Url;

use crate::components::relay_row::MessageWrapper;
use crate::components::text::title;
use crate::components::text_input_group::text_input_group;
use crate::components::{relay_row, RelayRow};
use crate::db::DbRelay;
use crate::icon::plus_icon;
use crate::net::{self, database, nostr_client, BackEndConnection, Connection};
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::MessageWrapper),
    BackEndEvent(net::Event),
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
    pub fn new(db_conn: &mut BackEndConnection<database::Message>) -> Self {
        db_conn.send(database::Message::FetchRelays);
        Self {
            relays: vec![],
            show_modal: false,
            add_relay_input: "".into(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        db_conn: &mut BackEndConnection<database::Message>,
        ns_conn: &mut BackEndConnection<nostr_client::Message>,
    ) -> Command<Message> {
        match message {
            Message::AddRelayInputChange(relay_addrs) => self.add_relay_input = relay_addrs,
            Message::CloseModal | Message::CancelButtonPressed => {
                self.add_relay_input = "".into();
                self.show_modal = false;
            }
            Message::OkButtonPressed => {
                match Url::try_from(self.add_relay_input.as_str()) {
                    Ok(url) => {
                        let db_relay = DbRelay::new(url);
                        ns_conn.send(nostr_client::Message::AddRelay(db_relay.clone()));
                        db_conn.send(database::Message::AddRelay(db_relay));
                        // ou eu adiciono nos dois canais
                        // ou eu faço um terceiro canal? que mandaria primeiro ao cliente e depois ao DB se confirmasse
                    }
                    Err(e) => {
                        // SOME VALIDATION TO THE USER
                        tracing::error!("{}", e);
                    }
                }
                self.add_relay_input = "".into();
                self.show_modal = false;
            }
            Message::OpenAddRelayModal => self.show_modal = true,

            Message::RelayMessage(msg) => {
                if let Some(row) = self.relays.iter_mut().find(|r| r.id == msg.from) {
                    let _ = row.update(msg, db_conn, ns_conn);
                }
            }

            Message::BackEndEvent(ev) => match ev {
                net::Event::DbEvent(db_event) => match db_event {
                    database::Event::RelayCreated(db_relay) => {
                        self.relays
                            .push(RelayRow::new(self.relays.len() as i32, db_relay, ns_conn))
                    }
                    database::Event::RelayUpdated(db_relay) => {
                        if let Some(row) = self
                            .relays
                            .iter_mut()
                            .find(|row| row.db_relay.url == db_relay.url)
                        {
                            return row
                                .update(
                                    MessageWrapper::new(
                                        row.id,
                                        relay_row::Message::RelayUpdated(db_relay),
                                    ),
                                    db_conn,
                                    ns_conn,
                                )
                                .map(Message::RelayMessage);
                        }
                    }
                    database::Event::RelayDeleted(db_relay) => {
                        self.relays.retain(|r| r.db_relay.url != db_relay.url);
                    }
                    database::Event::GotRelays(mut db_relays) => {
                        db_relays.sort_by(|a, b| a.url.cmp(&b.url));
                        self.relays = db_relays
                            .into_iter()
                            .enumerate()
                            .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay, ns_conn))
                            .collect();
                    }
                    _ => (),
                },
                net::Event::NostrClientEvent(ns_event) => {
                    if let nostr_client::Event::GotRelay(relay) = ns_event {
                        if let Some(relay) = relay {
                            if let Some(row) = self
                                .relays
                                .iter_mut()
                                .find(|r| r.db_relay.url == relay.url())
                            {
                                let _ = row.update(
                                    MessageWrapper {
                                        from: row.id,
                                        message: relay_row::Message::GotRelay(relay),
                                    },
                                    db_conn,
                                    ns_conn,
                                );
                            }
                        }
                    }
                }
                _ => (),
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
        let add_btn = button(
            row![text("Add").size(18), plus_icon().size(14)]
                .align_items(alignment::Alignment::Center)
                .spacing(2),
        )
        .padding(5)
        .on_press(Message::OpenAddRelayModal);
        let add_row = row![empty, add_btn];
        let content: Element<_> = container(column![title, add_row, relays])
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        Modal::new(self.show_modal, content, || {
            let add_relay_input = text_input_group(
                "Relay Address",
                "wss://my-relay.com",
                &self.add_relay_input,
                None,
                Message::AddRelayInputChange,
                None,
            );
            let modal_body: Element<_> = container(add_relay_input).into();
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
                //.width(Length::Shrink)
                .on_close(Message::CloseModal)
                .into()
        })
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal)
        .into()
    }
}

const CARD_MAX_WIDTH: f32 = 300.0;
