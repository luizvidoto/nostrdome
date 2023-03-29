use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::net::{self, Connection};

#[derive(Debug, Clone)]
pub enum Message {
    OnVerResize(u16),
    AddRelay,
    ShowRelays,
    NavSettingsPress,
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
                self.ver_divider_position = Some(position);
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
        }
    }
}
