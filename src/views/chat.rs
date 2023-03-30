use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::components;
use crate::components::chat_card::{self, ChatCard};
use crate::net::{self, Connection};

#[derive(Debug, Clone)]
pub enum Message {
    OnVerResize(u16),
    AddRelay,
    ShowRelays,
    NavSettingsPress,
    ChatCardMessage(components::chat_card::Message),
}

#[derive(Debug, Clone)]
pub struct State {
    ver_divider_position: Option<u16>,
    chats: Vec<chat_card::State>,
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
        }
    }
    pub fn view(&self) -> Element<Message> {
        let first = container(column![
            // button("Add Relay").on_press(Message::AddRelay),
            // button("Show Relay").on_press(Message::ShowRelays),
            scrollable(self.chats.iter().fold(column![].spacing(0), |col, card| {
                col.push(card.view().map(Message::ChatCardMessage))
            }))
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();

        let second = container(text("Second"))
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
            Message::AddRelay => {
                if let Err(e) = conn.send(net::Message::AddRelay("ws://192.168.15.119:8080".into()))
                {
                    println!("{}", e);
                }
            }
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
