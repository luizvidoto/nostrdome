use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::components::contact_card;
use crate::net::{self, BackEndConnection};
use crate::types::ChatMessage;

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
    contact_pubkey_active: Option<XOnlyPublicKey>,
    dm_msg: String,
    messages: Vec<ChatMessage>,
}
impl State {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            messages: vec![],
            ver_divider_position: None,
            contact_pubkey_active: None,
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
            self.contacts
                .iter()
                .fold(column![].spacing(0), |col, contact| {
                    col.push(contact.view().map(Message::ContactCardMessage))
                })
                .into()
        };
        let first = container(contact_list);
        let chat_content = self.messages.iter().fold(column![].spacing(5), |col, msg| {
            col.push(text(&msg.content))
        });
        let chat_row = scrollable(chat_content);
        let dm_msg_input = text_input("", &self.dm_msg).on_input(Message::DMNMessageChange);
        let dm_send_btn = button("Send DM").on_press(Message::DMSentPress);
        let msg_input_row = column![dm_msg_input, dm_send_btn].spacing(5);
        let second = container(column![chat_row, msg_input_row])
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y();
        let content = iced_aw::split::Split::new(
            first,
            second,
            self.ver_divider_position,
            iced_aw::split::Axis::Vertical,
            Message::OnVerResize,
        );

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
    pub fn backend_event(&mut self, event: net::Event, _back_conn: &mut BackEndConnection) {
        match event {
            net::Event::DatabaseSuccessEvent(kind) => match kind {
                net::DatabaseSuccessEventKind::NewDM(_message) => {
                    // verificar se estou na conversa?

                    // self.messages.push(message);
                    // self.messages
                    //     .sort_by(|a, b| a.created_at.cmp(&b.created_at))
                }
                _ => (),
            },
            net::Event::GotMessages(chat_msgs) => {
                self.messages = chat_msgs;
                self.messages
                    .sort_by(|a, b| a.created_at.cmp(&b.created_at))
            }
            net::Event::GotContacts(db_contacts) => {
                self.contacts = db_contacts
                    .iter()
                    .map(|c| contact_card::State::from_db_contact(c))
                    .collect();
            }
            _ => (),
        }
    }

    pub fn update(&mut self, message: Message, back_conn: &mut BackEndConnection) {
        match message {
            Message::AddContactPress => (),
            Message::DMNMessageChange(text) => {
                self.dm_msg = text;
            }
            Message::DMSentPress => {
                if let Some(pub_key) = self.contact_pubkey_active {
                    match back_conn.try_send(net::Message::SendDMTo((
                        pub_key.clone(),
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
                if position > 200 {
                    self.ver_divider_position = Some(position);
                } else if position <= 200 && position > 120 {
                    self.ver_divider_position = Some(200);
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowFullCard);
                    }
                } else if position <= 120 {
                    self.ver_divider_position = Some(80);
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowOnlyProfileImage);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ContactCardMessage(card_msg) => {
                match card_msg.clone() {
                    contact_card::Message::UpdateActiveId(hex_pub) => {
                        if let Some(active_hex) = self.contact_pubkey_active {
                            if active_hex != hex_pub {
                                back_conn.send(net::Message::FetchMessages(hex_pub));
                                self.messages = vec![];
                            }
                        }
                        self.dm_msg = "".into();
                        self.contact_pubkey_active = Some(hex_pub.clone());
                    }
                    _ => (),
                }
                for c in &mut self.contacts {
                    c.update(card_msg.clone());
                }
            }
        }
    }
}
