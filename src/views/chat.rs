use std::str::FromStr;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::components;
use crate::components::chat_card::{self, ChatCard};
use crate::components::text_input_group::text_input_group;
use crate::net::{self, Connection};
use crate::utils::parse_key;

#[derive(Debug, Clone)]
pub enum Message {
    DbEvent(net::Event),
    OnVerResize(u16),
    ShowRelays,
    NavSettingsPress,
    ChatCardMessage(components::chat_card::Message),
    ListOwnEvents,
    GetEventById(String),
    ShowPublicKey,
    DMNPubReceiverChange(String),
    DMNMessageChange(String),
    DMSentPress,
}

#[derive(Debug, Clone)]
pub struct State {
    ver_divider_position: Option<u16>,
    chats: Vec<chat_card::State>,
    dm_npub_receiver: String,
    dm_msg: String,
}
impl State {
    pub fn new() -> Self {
        let mut chats: Vec<chat_card::State> = vec![];
        for id in 0..10 {
            chats.push(chat_card::State::new(ChatCard::new(id)));
        }
        Self {
            ver_divider_position: None,
            chats,
            dm_npub_receiver: "".into(),
            dm_msg: "".into(),
        }
    }
    pub fn view(&self) -> Element<Message> {
        let first = container(column![scrollable(
            self.chats.iter().fold(column![].spacing(0), |col, card| {
                col.push(card.view().map(Message::ChatCardMessage))
            })
        )])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();

        let show_relay_btn = button("Show Relay").on_press(Message::ShowRelays);
        let get_own_events_btn = button("List Own Events").on_press(Message::ListOwnEvents);
        let show_public_btn = button("Show Public Key").on_press(Message::ShowPublicKey);
        let dm_receiver_pubkey_input = text_input_group(
            "DM To PubKey",
            "npub1...",
            &self.dm_npub_receiver,
            None,
            Message::DMNPubReceiverChange,
        );
        let dm_msg_input = text_input_group(
            "Message",
            "Hello friend...",
            &self.dm_msg,
            None,
            Message::DMNMessageChange,
        );
        let dm_send_btn = button("Send DM").on_press(Message::DMSentPress);
        let first_row = column![show_relay_btn, get_own_events_btn, show_public_btn].spacing(10);
        let second_row = column![dm_receiver_pubkey_input, dm_msg_input, dm_send_btn].spacing(5);
        let second = container(column![first_row, second_row])
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
    pub fn update(&mut self, message: Message, conn: &mut Connection) {
        match message {
            Message::DbEvent(_ev) => (),
            Message::DMNMessageChange(msg) => {
                self.dm_msg = msg;
            }
            Message::DMSentPress => match parse_key(self.dm_npub_receiver.clone()) {
                Ok(hex_key) => match XOnlyPublicKey::from_str(&hex_key) {
                    Ok(pub_key) => {
                        if let Err(e) =
                            conn.send(net::Message::SendDMTo((pub_key, self.dm_msg.clone())))
                        {
                            println!("{}", e);
                        }
                    }
                    Err(e) => {
                        println!("Invalid Public Key!");
                        println!("{}", e.to_string());
                    }
                },
                Err(e) => {
                    println!("Invalid Public Key!");
                    println!("{}", e.to_string());
                }
            },

            Message::DMNPubReceiverChange(npub) => {
                self.dm_npub_receiver = npub;
            }
            Message::ShowPublicKey => {
                if let Err(e) = conn.send(net::Message::ShowPublicKey) {
                    println!("{}", e);
                }
            }
            Message::GetEventById(ev_id) => {
                if let Err(e) = conn.send(net::Message::GetEventById(ev_id)) {
                    println!("{}", e);
                }
            }
            Message::ListOwnEvents => {
                if let Err(e) = conn.send(net::Message::ListOwnEvents) {
                    println!("{}", e);
                }
            }
            Message::OnVerResize(position) => {
                if position > 200 {
                    self.ver_divider_position = Some(position);
                } else if position <= 200 && position > 120 {
                    self.ver_divider_position = Some(200);
                    for c in &mut self.chats {
                        c.update(chat_card::Message::ShowFullCard);
                    }
                } else if position <= 120 {
                    self.ver_divider_position = Some(80);
                    for c in &mut self.chats {
                        c.update(chat_card::Message::ShowOnlyProfileImage);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ShowRelays => {
                if let Err(e) = conn.send(net::Message::ShowRelays) {
                    println!("{}", e);
                }
            }
            Message::ChatCardMessage(msg) => {
                for c in &mut self.chats {
                    c.update(msg.clone());
                }
            }
        }
    }
}
