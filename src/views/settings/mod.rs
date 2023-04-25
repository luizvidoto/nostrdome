use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text};
use iced::{Command, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr_sdk::{Metadata, Tag};

use crate::components::text_input_group::text_input_group;
use crate::components::{file_importer, FileImporter};
use crate::db::DbContact;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::types::UncheckedEvent;
use crate::utils::json_reader;
use crate::widget::Element;

mod account;
pub mod appearance;
mod backup;
mod contacts;
mod network;

#[derive(Debug, Clone)]
pub enum Message {
    AccountMessage(account::Message),
    AppearanceMessage(appearance::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    ContactsMessage(contacts::Message),
    MenuAccountPress,
    MenuAppearancePress,
    MenuNetworkPress,
    MenuBackupPress,
    MenuContactsPress,
    NavEscPress,

    // Modal Messages
    ModalPetNameInputChange(String),
    ModalPubKeyInputChange(String),
    ModalRecRelayInputChange(String),
    SubmitContact {
        petname: String,
        pubkey: String,
        rec_relay: String,
        is_edit: bool,
    },
    SaveImportedContacts(Vec<DbContact>),
    FileImporterMessage(file_importer::Message),
    CloseImportContactModal,
    CloseAddContactModal,
}

#[derive(Debug)]
pub enum MenuState {
    Account { state: account::State },
    Appearance { state: appearance::State },
    Network { state: network::State },
    Backup { state: backup::State },
    Contacts { state: contacts::State },
}

impl MenuState {
    fn account(_back_conn: &mut BackEndConnection) -> Self {
        let profile = Metadata::new();
        // conn.send(message)
        Self::Account {
            state: account::State::new(profile),
        }
    }
    fn appearance(selected_theme: Option<style::Theme>) -> Self {
        Self::Appearance {
            state: appearance::State::new(selected_theme),
        }
    }
    fn network(back_conn: &mut BackEndConnection) -> Self {
        Self::Network {
            state: network::State::new(back_conn),
        }
    }
    pub fn contacts(back_conn: &mut BackEndConnection) -> Self {
        Self::Contacts {
            state: contacts::State::new(back_conn),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        Self::account(back_conn)
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance { state } => state.view().map(Message::AppearanceMessage),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
            Self::Contacts { state } => state.view().map(Message::ContactsMessage),
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    menu_state: MenuState,
    modal_state: ModalState,
}
impl Settings {
    pub fn reset_inputs(&mut self) {
        self.modal_state = ModalState::Off;
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.menu_state {
            MenuState::Network { state } => state.subscription().map(Message::NetworkMessage),
            _ => Subscription::none(),
        }
    }
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::new(back_conn),
            modal_state: ModalState::Off,
        }
    }
    pub fn contacts(back_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::contacts(back_conn),
            modal_state: ModalState::Off,
        }
    }

    pub fn back_end_event(
        &mut self,
        event: net::Event,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match &mut self.menu_state {
            MenuState::Account { state } => state.update(account::Message::BackEndEvent(event)),
            MenuState::Appearance { state } => {
                state.update(appearance::Message::BackEndEvent(event))
            }
            MenuState::Network { state } => {
                return state
                    .update(network::Message::BackEndEvent(event), back_conn)
                    .map(Message::NetworkMessage);
            }
            MenuState::Backup { state } => state.update(backup::Message::BackEndEvent(event)),
            MenuState::Contacts { state } => {
                if let Some(received_msg) =
                    state.update(contacts::Message::BackEndEvent(event), back_conn)
                {
                    match received_msg {
                        contacts::Message::OpenEditContactModal(contact) => {
                            self.modal_state = ModalState::add_contact(Some(contact))
                        }
                        _ => (),
                    }
                }
            }
        }
        Command::none()
    }
    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> Command<Message> {
        match message {
            Message::AccountMessage(msg) => {
                if let MenuState::Account { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::AppearanceMessage(msg) => {
                if let MenuState::Appearance { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::NetworkMessage(msg) => {
                if let MenuState::Network { state } = &mut self.menu_state {
                    return state.update(msg, back_conn).map(Message::NetworkMessage);
                }
            }
            Message::BackupMessage(msg) => {
                if let MenuState::Backup { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::ContactsMessage(msg) => {
                if let MenuState::Contacts { state } = &mut self.menu_state {
                    match msg {
                        contacts::Message::OpenAddContactModal => {
                            self.modal_state = ModalState::add_contact(None);
                        }
                        contacts::Message::OpenImportContactModal => {
                            self.modal_state = ModalState::import_contacts();
                        }
                        other => {
                            if let Some(received_msg) = state.update(other, back_conn) {
                                match received_msg {
                                    contacts::Message::OpenEditContactModal(contact) => {
                                        self.modal_state = ModalState::add_contact(Some(contact))
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                }
            }
            Message::NavEscPress => (),
            Message::MenuAccountPress => {
                self.menu_state = MenuState::account(back_conn);
            }
            Message::MenuAppearancePress => {
                self.menu_state = MenuState::appearance(selected_theme);
            }
            Message::MenuNetworkPress => {
                self.menu_state = MenuState::network(back_conn);
            }
            Message::MenuBackupPress => {
                self.menu_state = MenuState::backup();
            }
            Message::MenuContactsPress => {
                self.menu_state = MenuState::contacts(back_conn);
            }
            other => return self.modal_state.update(other, back_conn),
        }
        Command::none()
    }

    pub fn view(&self) -> Element<Message> {
        let account_btn = button("Account")
            .style(match self.menu_state {
                MenuState::Account { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAccountPress);
        let appearance_btn = button("Appearance")
            .style(match self.menu_state {
                MenuState::Appearance { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAppearancePress);
        let network_btn = button("Network")
            .style(match self.menu_state {
                MenuState::Network { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuNetworkPress);
        let backup_btn = button("Backup")
            .style(match self.menu_state {
                MenuState::Backup { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuBackupPress);
        let contacts_btn = button("Contacts")
            .style(match self.menu_state {
                MenuState::Contacts { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuContactsPress);
        let esc_btn = button("Esc")
            .padding(10)
            .on_press(Message::NavEscPress)
            .style(style::Button::InactiveMenuBtn);
        let spacer = iced::widget::horizontal_space(Length::Fixed(3.0));
        let menubar = container(
            column![
                esc_btn,
                spacer,
                account_btn,
                appearance_btn,
                network_btn,
                backup_btn,
                contacts_btn
            ]
            .spacing(3),
        )
        .width(Length::FillPortion(1))
        .padding([10, 5]);

        let view_ct = container(self.menu_state.view())
            .padding([10, 5])
            .width(Length::FillPortion(3));

        let content: Element<_> = container(
            row![menubar, view_ct]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        self.modal_state.view(content)
    }
}

#[derive(Debug, Clone)]
enum ModalState {
    ContactDetails {
        modal_petname_input: String,
        modal_pubkey_input: String,
        modal_rec_relay_input: String,
        is_edit: bool,
    },
    ImportList {
        imported_contacts: Vec<DbContact>,
        file_importer: FileImporter,
    },
    Off,
}

impl ModalState {
    pub fn add_contact(contact: Option<DbContact>) -> Self {
        let (petname, pubkey, rec_relay, is_edit) = match contact {
            Some(c) => (
                c.get_petname().unwrap_or_else(|| "".into()),
                c.pubkey().to_string(),
                c.get_relay_url().unwrap_or_else(|| "".into()),
                true,
            ),
            None => ("".into(), "".into(), "".into(), false),
        };

        Self::ContactDetails {
            modal_petname_input: petname,
            modal_pubkey_input: pubkey,
            modal_rec_relay_input: rec_relay,
            is_edit,
        }
    }
    pub fn import_contacts() -> Self {
        Self::ImportList {
            imported_contacts: vec![],
            file_importer: FileImporter::new("/path/to/contacts.json")
                .file_filter("JSON File", &["json"]),
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match message {
            Message::CloseAddContactModal => *self = Self::Off,
            Message::CloseImportContactModal => *self = Self::Off,
            Message::SubmitContact {
                petname,
                pubkey,
                rec_relay,
                is_edit,
            } => match DbContact::from_str(&pubkey) {
                Ok(mut db_contact) => {
                    db_contact = db_contact.petname(&petname).relay_url(&rec_relay);
                    back_conn.send(match is_edit {
                        true => net::Message::UpdateContact(db_contact),
                        false => net::Message::AddContact(db_contact),
                    });
                    *self = ModalState::Off;
                }
                Err(_e) => {
                    tracing::error!("Every contact must have a valid pubkey.");
                }
            },
            Message::SaveImportedContacts(imported_contacts) => {
                back_conn.send(net::Message::ImportContacts(imported_contacts));
                *self = ModalState::Off;
            }
            Message::FileImporterMessage(msg) => {
                if let ModalState::ImportList {
                    imported_contacts,
                    file_importer,
                } = self
                {
                    if let Some(returned_msg) = file_importer.update(msg) {
                        handle_file_importer_message(returned_msg, imported_contacts);
                    }
                }
            }
            Message::ModalPetNameInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_petname_input,
                    ..
                } = self
                {
                    *modal_petname_input = text;
                }
            }
            Message::ModalPubKeyInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_pubkey_input, ..
                } = self
                {
                    *modal_pubkey_input = text;
                }
            }
            Message::ModalRecRelayInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_rec_relay_input,
                    ..
                } = self
                {
                    *modal_rec_relay_input = text;
                }
            }
            _ => (),
        }

        Command::none()
    }
    pub fn view<'a>(&'a self, underlay: Element<'a, Message>) -> Element<'a, Message> {
        let view: Element<_> = match &self {
            ModalState::ContactDetails {
                modal_petname_input,
                modal_pubkey_input,
                modal_rec_relay_input,
                is_edit,
            } => Modal::new(true, underlay, move || {
                let add_relay_input = text_input_group(
                    "Contact Name",
                    "",
                    modal_petname_input,
                    None,
                    Message::ModalPetNameInputChange,
                    None,
                );
                let add_pubkey_input = text_input_group(
                    "Contact PubKey",
                    "",
                    modal_pubkey_input,
                    None,
                    Message::ModalPubKeyInputChange,
                    None,
                );
                let add_rec_relay_input = text_input_group(
                    "Recommended Relay",
                    "",
                    modal_rec_relay_input,
                    None,
                    Message::ModalRecRelayInputChange,
                    None,
                );
                let modal_body: Element<_> =
                    column![add_relay_input, add_pubkey_input, add_rec_relay_input]
                        .spacing(4)
                        .into();
                let title = match is_edit {
                    true => text("Edit Contact"),
                    false => text("Add Contact"),
                };
                Card::new(title, modal_body)
                    .foot(
                        row![
                            button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                .width(Length::Fill)
                                .on_press(Message::CloseAddContactModal),
                            button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                .width(Length::Fill)
                                .on_press(Message::SubmitContact {
                                    petname: modal_petname_input.clone(),
                                    pubkey: modal_pubkey_input.clone(),
                                    rec_relay: modal_rec_relay_input.clone(),
                                    is_edit: is_edit.to_owned()
                                })
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill),
                    )
                    .max_width(ADD_CONTACT_MODAL_WIDTH)
                    .on_close(Message::CloseAddContactModal)
                    .into()
            })
            .backdrop(Message::CloseAddContactModal)
            .on_esc(Message::CloseAddContactModal)
            .into(),
            ModalState::ImportList {
                imported_contacts,
                file_importer,
            } => Modal::new(true, underlay, || {
                let importer_cp = file_importer.view().map(Message::FileImporterMessage);
                let found_contacts_txt = match imported_contacts.len() {
                    0 => text(""),
                    n => text(format!("Found contacts: {}", n)),
                };
                let stats_row = row![found_contacts_txt];

                let card_header = text("Import Contacts");
                let card_body = column![importer_cp, stats_row].spacing(4);
                let card_footer = row![
                    button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(Message::CloseImportContactModal),
                    button(text("Ok").horizontal_alignment(Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(Message::SaveImportedContacts(imported_contacts.clone()))
                ]
                .spacing(10)
                .padding(5)
                .width(Length::Fill);

                let card: Element<_> = Card::new(card_header, card_body)
                    .foot(card_footer)
                    .max_width(IMPORT_MODAL_WIDTH)
                    .on_close(Message::CloseImportContactModal)
                    .style(crate::style::Card::Default)
                    .into();

                card
            })
            .backdrop(Message::CloseImportContactModal)
            .on_esc(Message::CloseImportContactModal)
            .into(),
            ModalState::Off => underlay.into(),
        };

        view
    }
}

fn handle_file_importer_message(
    returned_msg: file_importer::Message,
    imported_contacts: &mut Vec<DbContact>,
) {
    if let file_importer::Message::OnChoose(path) = returned_msg {
        match json_reader::<String, UncheckedEvent>(path) {
            Ok(contact_event) => {
                if let nostr_sdk::event::Kind::ContactList = contact_event.kind {
                    update_imported_contacts(&contact_event.tags, imported_contacts);
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
}

fn update_imported_contacts(tags: &[Tag], imported_contacts: &mut Vec<DbContact>) {
    let (oks, errs): (Vec<_>, Vec<_>) = tags
        .iter()
        .map(DbContact::from_tag)
        .partition(Result::is_ok);

    let errors: Vec<_> = errs.into_iter().map(Result::unwrap_err).collect();

    for e in errors {
        tracing::error!("{}", e);
    }

    *imported_contacts = oks.into_iter().map(Result::unwrap).collect();
}

const ADD_CONTACT_MODAL_WIDTH: f32 = 400.0;
const IMPORT_MODAL_WIDTH: f32 = 400.0;
