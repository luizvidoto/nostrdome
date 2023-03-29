use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::net::{self, Connection};

#[derive(Debug, Clone)]
pub enum Message {
    OnVerResize(u16),
    AddRelay,
    ShowRelays,
}

#[derive(Debug, Clone)]
pub struct State {
    ver_divider_position: Option<u16>,
}
impl State {
    pub fn new() -> Self {
        Self {
            ver_divider_position: None,
        }
    }
    pub fn view(&self) -> Element<Message> {
        let first = container(column![
            text("First"),
            button("Add Relay").on_press(Message::AddRelay),
            button("Show Relay").on_press(Message::ShowRelays),
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
        iced_aw::split::Split::new(
            first,
            second,
            self.ver_divider_position,
            iced_aw::split::Axis::Vertical,
            Message::OnVerResize,
        )
        .into()
    }
    pub fn update(&mut self, message: Message, conn: &mut Connection) {
        match message {
            Message::OnVerResize(position) => {
                self.ver_divider_position = Some(position);
            }
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
        }
    }
}
