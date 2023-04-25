use chrono::{Datelike, NaiveDateTime};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{alignment, Command, Length};

use crate::components::contact_card;
use crate::db::DbContact;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::types::ChatMessage;
use crate::utils::send_icon;
use crate::widget::{Column, Element};
use once_cell::sync::Lazy;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

#[derive(Debug, Clone)]
pub enum Message {
    OnVerResize(u16),
    NavSettingsPress,
    ContactCardMessage(contact_card::Message),
    DMNMessageChange(String),
    DMSentPress,
    AddContactPress,
}

#[derive(Debug, Clone)]
pub struct State {
    ver_divider_position: Option<u16>,
    contacts: Vec<contact_card::State>,
    active_contact: Option<DbContact>,
    dm_msg: String,
    messages: Vec<ChatMessage>,
}
impl State {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            messages: vec![],
            ver_divider_position: Some(300),
            active_contact: None,
            dm_msg: "".into(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        // let first = container(column![scrollable(
        //     self.chats.iter().fold(column![].spacing(0), |col, card| {
        //         col.push(card.view().map(Message::ChatCardMessage))
        //     })
        // )])
        // .width(Length::Fill)
        // .height(Length::Fill)
        // .center_x()
        // .center_y();

        let contact_list: Element<_> = if self.contacts.is_empty() {
            button("Add Contact")
                .on_press(Message::AddContactPress)
                .into()
        } else {
            scrollable(
                self.contacts
                    .iter()
                    .fold(column![].spacing(0), |col, contact| {
                        col.push(contact.view().map(Message::ContactCardMessage))
                    }),
            )
            .id(SCROLLABLE_ID.clone())
            .into()
        };
        let first = container(contact_list);
        let (chat_content, _) = self.messages.iter().fold(
            (column![], None),
            |(mut col, last_date): (Column<'_, _>, Option<NaiveDateTime>), msg| {
                match (last_date, msg.created_at) {
                    (None, msg_date) => {
                        col = col.push(chat_day_divider(msg_date.clone()));
                    }
                    (Some(last_date), msg_date) => {
                        if last_date.day() != msg_date.day() {
                            col = col.push(chat_day_divider(msg_date.clone()));
                        }
                    }
                }

                (col.push(chat_message(&msg)), Some(msg.created_at))
            },
        );
        let chat_messages = scrollable(chat_content).height(Length::Fill);
        let message_input = text_input("Write a message...", &self.dm_msg)
            .on_submit(Message::DMSentPress)
            .on_input(Message::DMNMessageChange);
        let send_btn = button(send_icon().style(style::Text::Primary))
            .style(style::Button::Invisible)
            .on_press(Message::DMSentPress);
        let msg_input_row = container(row![message_input, send_btn].spacing(5)).padding([10, 5]);

        let second_content: Element<_> = if self.active_contact.is_some() {
            column![chat_messages, msg_input_row].into()
        } else {
            container(text("Select a chat to start messaging"))
                .center_x()
                .center_y()
                .into()
        };

        let second = container(second_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::Container::ChatContainer)
            .center_x()
            .center_y();
        let content = iced_aw::split::Split::new(
            first,
            second,
            self.ver_divider_position,
            iced_aw::split::Axis::Vertical,
            Message::OnVerResize,
        )
        .spacing(1.0)
        .min_size_second(300);

        let search_input = container(text("Search")).padding(10);
        let settings_btn = button("Settings")
            .padding(10)
            .on_press(Message::NavSettingsPress);
        let empty = container(text("")).width(Length::Fill);
        let navbar = row![search_input, empty, settings_btn]
            .width(Length::Fill)
            .padding(10)
            .spacing(10);

        column![navbar, content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    pub fn backend_event(
        &mut self,
        event: net::Event,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match event {
            net::Event::DatabaseSuccessEvent(kind) => {
                match kind {
                    net::DatabaseSuccessEventKind::SentDM((_id, db_contact, msg)) => {
                        self.update_contact(db_contact.clone());

                        if self.active_contact.as_ref() == Some(&db_contact) {
                            // estou na conversa
                            self.messages.push(msg);
                            return scrollable::snap_to(
                                SCROLLABLE_ID.clone(),
                                scrollable::RelativeOffset::END,
                            );
                        } else {
                            // não estou na conversa
                            tracing::error!("Impossible to send message outside chat");
                        }
                    }
                    net::DatabaseSuccessEventKind::ReceivedDM((db_contact, msg)) => {
                        self.update_contact(db_contact.clone());

                        if self.active_contact.as_ref() == Some(&db_contact) {
                            // estou na conversa
                            self.messages.push(msg.clone());
                            return scrollable::snap_to(
                                SCROLLABLE_ID.clone(),
                                scrollable::RelativeOffset::END,
                            );
                        } else {
                            // não estou na conversa
                            back_conn.send(net::Message::UpdateUnseenCount(db_contact))
                        }
                    }

                    net::DatabaseSuccessEventKind::NewDMAndContact((db_contact, _)) => self
                        .contacts
                        .push(contact_card::State::from_db_contact(&db_contact)),

                    net::DatabaseSuccessEventKind::ContactUpdated(db_contact) => {
                        self.update_contact(db_contact);
                    }

                    _ => (),
                }
            }
            net::Event::GotChatMessages((db_contact, chat_msgs)) => {
                self.update_contact(db_contact.clone());

                if self.active_contact.as_ref() == Some(&db_contact) {
                    self.messages = chat_msgs;
                    self.messages
                        .sort_by(|a, b| a.created_at.cmp(&b.created_at));

                    return scrollable::snap_to(
                        SCROLLABLE_ID.clone(),
                        scrollable::RelativeOffset::END,
                    );
                }
            }
            net::Event::GotContacts(db_contacts) => {
                self.contacts = db_contacts
                    .iter()
                    .map(|c| contact_card::State::from_db_contact(c))
                    .collect();
            }
            _ => (),
        }

        Command::none()
    }

    fn update_contact(&mut self, db_contact: DbContact) {
        // change active to be an ID again...

        if let Some(found_card) = self
            .contacts
            .iter_mut()
            .find(|c| c.contact.pubkey() == db_contact.pubkey())
        {
            found_card.update(contact_card::Message::ContactUpdated(db_contact.clone()));
        }

        if self.active_contact.as_ref() == Some(&db_contact) {
            self.active_contact = Some(db_contact);
        }
    }

    pub fn update(&mut self, message: Message, back_conn: &mut BackEndConnection) {
        match message {
            Message::AddContactPress => (),
            Message::DMNMessageChange(text) => {
                self.dm_msg = text;
            }
            Message::DMSentPress => {
                if let Some(contact) = &self.active_contact {
                    match back_conn.try_send(net::Message::SendDMTo((
                        contact.to_owned(),
                        self.dm_msg.clone(),
                    ))) {
                        Ok(_) => {
                            self.dm_msg = "".into();
                        }
                        Err(e) => {
                            tracing::error!("{}", e);
                        }
                    }
                }
            }

            Message::OnVerResize(position) => {
                if position > 200 && position < 400 {
                    self.ver_divider_position = Some(position);
                } else if position <= 200 && position > PIC_WIDTH {
                    self.ver_divider_position = Some(200);
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowFullCard);
                    }
                } else if position <= PIC_WIDTH {
                    self.ver_divider_position = Some(80);
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowOnlyProfileImage);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ContactCardMessage(card_msg) => {
                if let contact_card::Message::UpdateActiveContact(contact) = &card_msg {
                    if self.active_contact.as_ref() != Some(&contact) {
                        back_conn.send(net::Message::FetchMessages(contact.clone()));
                        self.messages = vec![];
                    }
                    self.dm_msg = "".into();
                    self.active_contact = Some(contact.clone());
                }

                for c in &mut self.contacts {
                    c.update(card_msg.clone());
                }
            }
        }
    }
}

fn chat_message<Message: 'static>(chat_msg: &ChatMessage) -> Element<'static, Message> {
    let chat_alignment = match chat_msg.is_from_user {
        false => alignment::Horizontal::Left,
        true => alignment::Horizontal::Right,
    };

    let container_style = if chat_msg.is_from_user {
        style::Container::SentMessage
    } else {
        style::Container::ReceivedMessage
    };

    let time_str = chat_msg.created_at.time().format("%H:%M").to_string();
    let data_cp = column![
        // container(text("")).height(10.0),
        container(text(&time_str).style(style::Text::Placeholder).size(14))
    ];

    let msg_content = text(&chat_msg.content).size(18);

    let message_container = container(row![msg_content, data_cp].spacing(5))
        .padding([5, 10])
        .style(container_style);

    container(message_container)
        .width(Length::Fill)
        .center_y()
        .align_x(chat_alignment)
        .padding([2, 20])
        .into()
}

fn chat_day_divider<Message: 'static>(date: NaiveDateTime) -> Element<'static, Message> {
    let text_container = container(text(date.format("%Y-%m-%d").to_string()))
        .style(style::Container::ChatDateDivider)
        .padding([5, 10]);
    container(text_container)
        .width(Length::Fill)
        .center_x()
        .center_y()
        .into()
}

const PIC_WIDTH: u16 = 50;
