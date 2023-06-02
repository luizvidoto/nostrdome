use crate::components::copy_btn;
use crate::components::text::title;
use crate::db::DbEvent;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::utils::hide_string;
use crate::{db::DbContact, widget::Element};
use iced::widget::{button, column, row, text};
use iced::{clipboard, Alignment, Command, Length};
use nostr::prelude::ToBech32;
use nostr::Keys;

#[derive(Debug, Clone)]
pub enum Listener {
    Contacts,
    Messages,
}

#[derive(Debug, Clone)]
pub enum Message {
    ExportContacts,
    ExportMessages,
    HidePublicKey,
    ShowPublicKey,
    CopyPublicKey,
    ShowSecretKey,
    HideSecretKey,
    CopySecretKey,
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
    public_key_visible: bool,
    secret_key_visible: bool,
    keys: Option<Keys>,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchContacts);
        conn.send(net::ToBackend::FetchAllMessageEvents);
        conn.send(net::ToBackend::FetchKeys);
        Self {
            contacts: Vec::new(),
            messages: Vec::new(),
            contacts_state: LoadingState::Idle,
            messages_state: LoadingState::Idle,
            listening_to: None,
            public_key_visible: false,
            secret_key_visible: false,
            keys: None,
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
            BackendEvent::GotKeys(keys) => self.keys = Some(keys),
            _ => (),
        }
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        let mut commands = vec![];

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
            Message::ShowPublicKey => {
                self.public_key_visible = true;
            }
            Message::HidePublicKey => {
                self.public_key_visible = false;
            }
            Message::CopyPublicKey => {
                if let Some(keys) = &self.keys {
                    commands.push(clipboard::write(
                        keys.public_key()
                            .to_bech32()
                            .unwrap_or(keys.public_key().to_string()),
                    ))
                }
            }
            Message::ShowSecretKey => {
                self.secret_key_visible = true;
            }
            Message::HideSecretKey => {
                self.secret_key_visible = false;
            }
            Message::CopySecretKey => {
                if let Some(keys) = &self.keys {
                    if let Ok(secret_key) = keys.secret_key() {
                        commands.push(clipboard::write(
                            secret_key
                                .to_bech32()
                                .unwrap_or(secret_key.display_secret().to_string()),
                        ))
                    } else {
                        tracing::error!("Error: secret key is not available");
                    }
                }
            }
        }

        Command::batch(commands)
    }

    pub fn view(&self) -> Element<Message> {
        let page_title = title("Backup");

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

        let keys_title = title("Nostr Keys");
        let mut keys_group = column![keys_title,].spacing(10);
        if let Some(keys) = &self.keys {
            keys_group = keys_group.push(self.make_public_key(keys));
            keys_group = keys_group.push(self.make_secret_key(keys));
        } else {
            keys_group = keys_group.push(text("Loading keys..."));
        };

        column![page_title, contacts_group, messages_group, keys_group]
            .spacing(10)
            .into()
    }

    fn make_public_key(&self, keys: &Keys) -> Element<Message> {
        let public_key_btn = if self.public_key_visible {
            button("Hide Public Key").on_press(Message::HidePublicKey)
        } else {
            button("Show Public Key").on_press(Message::ShowPublicKey)
        };

        let public_key_ct: Element<_> = if self.public_key_visible {
            let public_key_txt = text(hide_string(
                &keys
                    .public_key()
                    .to_bech32()
                    .unwrap_or(keys.public_key().to_string()),
                OPEN_VALUE,
            ));

            row![
                public_key_txt,
                copy_btn("Copy Public Key", Message::CopyPublicKey)
            ]
            .align_items(Alignment::Center)
            .spacing(5)
            .into()
        } else {
            text("********************************************************").into()
        };

        column![public_key_btn, public_key_ct].into()
    }

    fn make_secret_key(&self, keys: &Keys) -> Element<Message> {
        if let Ok(secret_key) = keys.secret_key() {
            let secret_key_btn = if self.secret_key_visible {
                button("Hide Secret Key").on_press(Message::HideSecretKey)
            } else {
                button("Show Secret Key").on_press(Message::ShowSecretKey)
            };

            let secret_key_ct: Element<_> = if self.secret_key_visible {
                let secret_key_txt = text(hide_string(
                    &secret_key
                        .to_bech32()
                        .unwrap_or(secret_key.display_secret().to_string()),
                    OPEN_VALUE,
                ));

                row![
                    secret_key_txt,
                    copy_btn("Copy Secret Key", Message::CopySecretKey)
                ]
                .align_items(Alignment::Center)
                .spacing(5)
                .into()
            } else {
                text("********************************************************").into()
            };

            column![secret_key_btn, secret_key_ct].into()
        } else {
            text("Error: secret key is not available").into()
        }
    }
}

const OPEN_VALUE: usize = 16;
