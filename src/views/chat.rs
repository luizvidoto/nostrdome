use std::str::FromStr;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Keys, Kind};

use crate::components;
use crate::components::chat_card::{self, ChatCard};
use crate::components::text_input_group::text_input_group;
use crate::net::database::DbConnection;
use crate::net::nostr::NostrConnection;
use crate::net::{self};

#[derive(Debug, Clone)]
pub enum Message {
    OnVerResize(u16),
    ShowRelays,
    NavSettingsPress,
    ChatCardMessage(components::chat_card::Message),
    GetEventById(String),
    DMNMessageChange(String),
    DMSentPress,
    DmToPubInputChange(String),
}

#[derive(Debug, Clone)]
pub struct State {
    ver_divider_position: Option<u16>,
    chats: Vec<chat_card::State>,
    dm_hex_pub_receiver: Option<XOnlyPublicKey>,
    dm_msg: String,
    messages: Vec<String>,
    dm_to_pub: String,
}
impl State {
    pub fn new(keys: &Keys, db_conn: &mut DbConnection) -> Self {
        if let Err(e) = db_conn.send(net::database::Message::FetchMessages(keys.clone())) {
            tracing::error!("{}", e);
        }

        let mut chats: Vec<chat_card::State> = vec![];
        let hex_pubs = vec![
            "8860df7d3b24bfb40fe5bdd2041663d35d3e524ce7376628aa55a7d3e624ac46",
            "9e45b5e573adfb70be9f81e6f19e3df334fa24b3a7273859104d399ccbf64e94",
        ];
        for pb_key in hex_pubs {
            let name = pb_key[0..6].to_owned();
            let profile_image = "https://picsum.photos/60/60";
            chats.push(chat_card::State::new(ChatCard::new(
                pb_key,
                name,
                profile_image,
            )));
        }
        Self {
            messages: vec![],
            ver_divider_position: None,
            chats,
            dm_hex_pub_receiver: None,
            dm_msg: "".into(),
            dm_to_pub: "".into(),
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

        let to_pub_input = text_input_group(
            "DM To",
            "npub...",
            &self.dm_to_pub,
            None,
            Message::DmToPubInputChange,
        );
        let first = container(to_pub_input);

        let chat_content = self
            .messages
            .iter()
            .fold(column![].spacing(5), |col, msg| col.push(text(msg)));
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
    pub fn db_event(
        &mut self,
        event: net::database::Event,
        _db_conn: &mut DbConnection,
        _nostr_conn: &mut NostrConnection,
    ) {
        match event {
            net::database::Event::DatabaseSuccessEvent(kind) => match kind {
                net::database::DatabaseSuccessEventKind::NewDM(message) => {
                    self.messages.push(message);
                }
                _ => (),
            },
            net::database::Event::GotMessages(messages) => {
                tracing::info!("{:?}", &messages);
                self.messages = messages;
            }
            net::database::Event::GotNewMessage(message) => {
                self.messages.push(message);
            }
            _ => (),
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        _db_conn: &mut DbConnection,
        nostr_conn: &mut NostrConnection,
    ) {
        match message {
            Message::DmToPubInputChange(changed) => {
                self.dm_to_pub = changed;
            }
            Message::DMNMessageChange(msg) => {
                self.dm_msg = msg;
            }
            Message::DMSentPress => {
                match XOnlyPublicKey::from_str(&self.dm_to_pub) {
                    Ok(pub_key) => {
                        match nostr_conn.send(net::nostr::Message::SendDMTo((
                            pub_key,
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
                    Err(e) => {
                        tracing::error!("{}", e);
                    }
                }

                // if let Some(pub_key) = self.dm_hex_pub_receiver {

                // }
            }

            Message::GetEventById(_ev_id) => {
                // if let Err(e) = db_conn.send(net::Message::GetEventById(ev_id)) {
                //     println!("{}", e);
                // }
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
                // if let Err(e) = db_conn.send(net::Message::ShowRelays) {
                //     println!("{}", e);
                // }
            }
            Message::ChatCardMessage(msg) => {
                match msg.clone() {
                    chat_card::Message::UpdateActiveId(hex_pub) => {
                        match XOnlyPublicKey::from_str(&hex_pub) {
                            Ok(hex_pub) => {
                                self.dm_hex_pub_receiver = Some(hex_pub.clone());
                            }
                            Err(e) => {
                                tracing::error!("{}", e);
                            }
                        }
                    }
                    _ => (),
                }
                for c in &mut self.chats {
                    c.update(msg.clone());
                }
            }
        }
    }
}
