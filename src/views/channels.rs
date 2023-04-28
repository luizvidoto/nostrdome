use crate::{
    components::text::title,
    net::{self, database, nostr_client, BackEndConnection, Connection},
    style,
    widget::Element,
};
use iced::{
    widget::{button, column, container, row, text, Space},
    Command, Length,
};

#[derive(Debug, Clone)]
pub enum Message {
    CreateChannel,
    GoToChat,
}

#[derive(Debug, Clone)]
pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }

    pub fn backend_event(
        &mut self,
        event: net::Event,
        _db_conn: &mut BackEndConnection<database::Message>,
    ) -> Command<Message> {
        if let net::Event::NostrClientEvent(ns_event) = event {
            if let nostr_client::Event::ChannelCreated(event_id) = ns_event {
                println!("*** CHANNEL CREATED ***");
                println!("{event_id}");
            }
        }
        Command::none()
    }
    pub fn update(&mut self, message: Message, db_conn: &mut BackEndConnection<database::Message>) {
        match message {
            Message::GoToChat => (),
            Message::CreateChannel => {
                println!("Creating channel");
                // db_conn.send(database::Message::CreateChannel);
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let nav_bar = container(
            row![
                Space::with_width(Length::Fill),
                button("Create New"),
                button("Go To Chat").on_press(Message::GoToChat),
            ]
            .spacing(5),
        )
        .padding(10)
        .width(Length::Fill)
        .height(NAVBAR_HEIGHT)
        .style(style::Container::Default);

        let title = title("Channels");
        let channels = (0..10).into_iter().fold(column![].spacing(5), |col, num| {
            col.push(text(format!("Channel {}", num)))
        });
        let view_ct = container(column![title, channels].spacing(10))
            .width(Length::Fill)
            .height(Length::Fill);

        column![nav_bar, view_ct].into()
    }
}

const NAVBAR_HEIGHT: f32 = 50.0;
