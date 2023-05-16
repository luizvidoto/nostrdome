use crate::components::text::title;
use crate::db::DbEvent;
use crate::net::{self, events::Event, BackEndConnection};
use crate::{db::DbContact, widget::Element};
use iced::widget::{button, column, row, text};
use rfd::FileDialog;

#[derive(Debug, Clone)]
pub enum Message {
    ExportContacts,
    ExportMessages,
}
#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Loading,
    Success,
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    messages: Vec<DbEvent>,
    contacts_state: LoadingState,
    messages_state: LoadingState,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchContacts);
        conn.send(net::Message::FetchAllMessageEvents);
        Self {
            contacts: Vec::new(),
            messages: Vec::new(),
            contacts_state: LoadingState::Idle,
            messages_state: LoadingState::Idle,
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
            Event::ExportedContactsSucessfully => {
                self.contacts_state = LoadingState::Success;
            }
            Event::ExportedMessagesSucessfully => {
                self.messages_state = LoadingState::Success;
            }
            _ => (),
        }
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::ExportContacts => {
                self.contacts_state = LoadingState::Loading;
                if let Some(path) = FileDialog::new().set_directory("/").save_file() {
                    conn.send(net::Message::ExportContacts(path))
                }
            }
            Message::ExportMessages => {
                self.messages_state = LoadingState::Loading;
                if let Some(path) = FileDialog::new().set_directory("/").save_file() {
                    conn.send(net::Message::ExportMessages((self.messages.clone(), path)));
                }
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Backup");

        let mut export_contacts_btn = button("Export contacts");
        match self.contacts_state {
            LoadingState::Idle => {
                export_contacts_btn = export_contacts_btn.on_press(Message::ExportContacts)
            }
            LoadingState::Loading => export_contacts_btn = button("Loading..."),
            LoadingState::Success => export_contacts_btn = button("Saved!"),
        }
        let contacts_group = column![
            row![text(format!("Number of contacts: {}", self.contacts.len())),].spacing(4),
            export_contacts_btn,
        ]
        .spacing(5);

        let mut export_messages_btn = button("Export messages");
        match self.messages_state {
            LoadingState::Idle => {
                export_messages_btn = export_messages_btn.on_press(Message::ExportMessages)
            }
            LoadingState::Loading => export_messages_btn = button("Loading..."),
            LoadingState::Success => export_messages_btn = button("Saved!"),
        }
        let messages_group = column![
            row![text(format!("Number of messages: {}", self.messages.len())),].spacing(4),
            export_messages_btn,
        ]
        .spacing(5);

        column![title, contacts_group, messages_group]
            .spacing(10)
            .into()
    }
}
