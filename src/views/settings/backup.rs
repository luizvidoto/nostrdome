use crate::components::text::title;
use crate::db::DbMessage;
use crate::net::{self, events::Event, BackEndConnection};
use crate::{db::DbContact, widget::Element};
use iced::widget::{column, row, text};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    messages: Vec<DbMessage>,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchContacts);
        conn.send(net::Message::FetchAllMessages);
        Self {
            contacts: Vec::new(),
            messages: Vec::new(),
        }
    }

    pub fn backend_event(&mut self, event: Event, _conn: &mut BackEndConnection) {
        match event {
            Event::GotContacts(contact_list) => {
                self.contacts = contact_list;
            }
            Event::GotAllMessages(all_messages) => {
                self.messages = all_messages;
            }
            _ => (),
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {}
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Backup");
        let contacts_group =
            row![text(format!("Number of contacts: {}", self.contacts.len())),].spacing(4);
        let messages_group =
            row![text(format!("Number of messages: {}", self.messages.len())),].spacing(4);

        column![title, contacts_group, messages_group]
            .spacing(10)
            .into()
    }
}
