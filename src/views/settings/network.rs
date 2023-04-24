use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text};
use iced::{Command, Element, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr_sdk::Url;

use crate::components::text::title;
use crate::components::text_input_group::text_input_group;
use crate::components::{relay_row, RelayRow};
use crate::net::{self, BackEndConnection};

const CARD_MAX_WIDTH: f32 = 300.0;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::Message),
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
        back_conn: &mut BackEndConnection,
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
                        back_conn.send(net::Message::AddRelay(url));
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
            Message::BackEndEvent(ev) => match ev {
                net::Event::GotRelays(mut rls) => {
                    rls.sort_by(|a, b| a.url().cmp(&b.url()));
                    self.relays = rls
                        .into_iter()
                        .filter_map(|r| RelayRow::new(r).ok())
                        .collect();
                }
                net::Event::DatabaseSuccessEvent(kind) => match kind {
                    net::DatabaseSuccessEventKind::RelayCreated
                    | net::DatabaseSuccessEventKind::RelayDeleted
                    | net::DatabaseSuccessEventKind::RelayUpdated => {
                        back_conn.send(net::Message::FetchRelays);
                    }
                    _ => (),
                },
                _ => (),
            },
            Message::RelayMessage(msg) => {
                self.relays.iter_mut().for_each(|r| {
                    let _ = r.update(msg.clone(), back_conn);
                });
            }
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
