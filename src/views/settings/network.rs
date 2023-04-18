use iced::alignment::Horizontal;
use iced::time::Instant;
use iced::widget::{button, column, container, row, text};
use iced::{Command, Element, Length, Subscription};
use iced_aw::{Card, Modal};

use crate::components::text::title;
use crate::components::text_input_group::text_input_group;
use crate::components::{relay_row, RelayRow};
use crate::db::DbRelay;
use crate::net::{self, BackEndConnection};
use crate::types::RelayUrl;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::Message),
    BackEndEvent(net::Event),
    OpenAddRelayModal,
    CancelButtonPressed,
    OkButtonPressed,
    CloseModal,
    AddRelayInputChange(String),
    Tick(Instant),
}

#[derive(Debug, Clone)]
pub struct State {
    relays: Vec<RelayRow>,
    show_modal: bool,
    add_relay_input: String,
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        // iced::time::every(Duration::from_secs(1)).map(Message::Tick)
        let relay_subs: Vec<_> = self
            .relays
            .iter()
            .map(|r| r.subscription().map(Message::RelayMessage))
            .collect();
        iced::Subscription::batch(relay_subs)
    }
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchRelays);
        Self {
            relays: vec![],
            show_modal: false,
            add_relay_input: "".into(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        db_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match message {
            Message::Tick(_) => {
                // for r in self.relays {

                // }
            }
            Message::AddRelayInputChange(relay_addrs) => self.add_relay_input = relay_addrs,
            Message::CloseModal | Message::CancelButtonPressed => {
                self.add_relay_input = "".into();
                self.show_modal = false;
            }
            Message::OkButtonPressed => {
                match RelayUrl::try_from_str(&self.add_relay_input) {
                    Ok(url) => {
                        db_conn.send(net::Message::AddRelay(DbRelay::new(url)));
                    }
                    Err(e) => {
                        tracing::error!("{}", e);
                    }
                }
                self.add_relay_input = "".into();
                self.show_modal = false;
            }
            Message::OpenAddRelayModal => self.show_modal = true,
            Message::BackEndEvent(ev) => match ev {
                net::Event::GotRelays(rls) => {
                    self.relays = rls
                        .into_iter()
                        .filter_map(|r| RelayRow::new(r).ok())
                        .collect();
                }
                net::Event::DatabaseSuccessEvent(kind) => match kind {
                    net::DatabaseSuccessEventKind::RelayCreated
                    | net::DatabaseSuccessEventKind::RelayDeleted
                    | net::DatabaseSuccessEventKind::RelayUpdated => {
                        db_conn.send(net::Message::FetchRelays);
                    }
                    _ => (),
                },
                _ => (),
            },
            Message::RelayMessage(msg) => match msg.clone() {
                relay_row::Message::None => (),
                relay_row::Message::DeleteRelay(relay_url) => {
                    db_conn.send(net::Message::DeleteRelay(relay_url));
                }
                relay_row::Message::ToggleRead => {
                    // db_relay.read = !db_relay.read;
                    // db_relay.opts().set_read(!db_relay.opts().read());
                    // db_conn.send(net::Message::UpdateRelay(db_relay));
                }
                relay_row::Message::ToggleWrite => {
                    // db_relay.write = !db_relay.write;
                    // db_relay.opts().set_write(!db_relay.opts().write());
                    // db_conn.send(net::Message::UpdateRelay(db_relay));
                }
                relay_row::Message::ToggleAdvertise => {
                    // db_relay.advertise = !db_relay.advertise;
                    // db_conn.send(net::Message::UpdateRelay(db_relay));
                }
                relay_row::Message::UpdateStatus(_) => {
                    self.relays.iter_mut().for_each(|r| {
                        r.update(msg.clone());
                    });
                }
                relay_row::Message::Ready(mut conn) => {
                    conn.send(relay_row::Input::GetStatus);
                }
            },
        }
        Command::none()
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Network");
        let header = column![RelayRow::view_header().map(Message::RelayMessage)];
        let relays = self.relays.iter().fold(header, |col, relay| {
            col.push(relay.view().map(Message::RelayMessage))
        });
        let empty = container(text("")).width(Length::Fill);
        let add_btn = button("Add").on_press(Message::OpenAddRelayModal);
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
                .max_width(300.0)
                //.width(Length::Shrink)
                .on_close(Message::CloseModal)
                .into()
        })
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal)
        .into()
    }
}
