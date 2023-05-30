use crate::components::text::title;
use crate::db::DbEvent;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::{db::DbContact, widget::Element};
use iced::widget::{button, column, row, text};

#[derive(Debug, Clone)]
pub enum Listener {
    Contacts,
    Messages,
}

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
    listening_to: Option<Listener>,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchContacts);
        conn.send(net::ToBackend::FetchAllMessageEvents);
        Self {
            contacts: Vec::new(),
            messages: Vec::new(),
            contacts_state: LoadingState::Idle,
            messages_state: LoadingState::Idle,
            listening_to: None,
        }
    }

    pub fn backend_event(&mut self, event: BackendEvent, _conn: &mut BackEndConnection) {
        match event {
            BackendEvent::GotContacts(contact_list) => {
                self.contacts = contact_list;
            }
            BackendEvent::GotAllMessages(all_messages) => {
                self.messages = all_messages;
            }
            BackendEvent::RFDSavedFile(_path) => match self.listening_to {
                Some(Listener::Contacts) => self.contacts_state = LoadingState::Success,
                Some(Listener::Messages) => self.messages_state = LoadingState::Success,
                None => (),
            },
            BackendEvent::RFDCancelPick => match self.listening_to {
                Some(Listener::Contacts) => self.contacts_state = LoadingState::Idle,
                Some(Listener::Messages) => self.messages_state = LoadingState::Idle,
                None => (),
            },
            _ => (),
        }
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::ExportContacts => {
                self.contacts_state = LoadingState::Loading;
                self.listening_to = Some(Listener::Contacts);
                conn.send(net::ToBackend::ExportContacts);
            }
            Message::ExportMessages => {
                self.messages_state = LoadingState::Loading;
                self.listening_to = Some(Listener::Messages);
                conn.send(net::ToBackend::ExportMessages(self.messages.clone()));
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
