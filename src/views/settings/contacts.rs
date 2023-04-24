use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::Length;
use iced_aw::{Card, Modal};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Kind, Tag};

use crate::components::text_input_group::text_input_group;
use crate::components::{contact_row, file_importer, ContactRow, FileImporter};
use crate::net::BackEndConnection;
use crate::types::UncheckedEvent;
use crate::utils::json_reader;
use crate::widget::Element;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    SubmitContact {
        petname: String,
        pubkey: String,
        rec_relay: String,
        is_edit: bool,
    },
    CloseAddContactModal,
    OpenAddContactModal,
    BackEndEvent(net::Event),
    ModalPetNameInputChange(String),
    ModalPubKeyInputChange(String),
    ModalRecRelayInputChange(String),
    DeleteContact(XOnlyPublicKey),
    ContactRowMessage(contact_row::Message),
    FileImporterMessage(file_importer::Message),
    OpenImportContactModal,
    CloseImportContactModal,
    SaveImportedContacts(Vec<DbContact>),
    SearchContactInputChange(String),
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
                c.petname.unwrap_or_else(|| "".into()),
                c.pubkey.to_string(),
                c.relay_url.unwrap_or_else(|| "".into()),
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
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    modal_state: ModalState,
    search_contact_input: String,
}
impl State {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            modal_state: ModalState::Off,
            search_contact_input: "".into(),
        }
    }

    pub fn reset_inputs(&mut self) {
        self.modal_state = ModalState::Off;
    }

    pub fn update(&mut self, message: Message, back_conn: &mut BackEndConnection) {
        match message {
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ContactRowMessage(ct_msg) => match ct_msg {
                contact_row::Message::DeleteContact(pubkey) => {
                    back_conn.send(net::Message::DeleteContact(pubkey))
                }
                contact_row::Message::EditContact(contact) => {
                    self.modal_state = ModalState::add_contact(Some(contact));
                }
            },
            Message::DeleteContact(pubkey) => {
                back_conn.send(net::Message::DeleteContact(pubkey));
            }
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
                    self.reset_inputs();
                }
                Err(_e) => {
                    tracing::error!("Every contact must have a valid pubkey.");
                }
            },
            Message::ModalPetNameInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_petname_input,
                    ..
                } = &mut self.modal_state
                {
                    *modal_petname_input = text;
                }
            }
            Message::ModalPubKeyInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_pubkey_input, ..
                } = &mut self.modal_state
                {
                    *modal_pubkey_input = text;
                }
            }
            Message::ModalRecRelayInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_rec_relay_input,
                    ..
                } = &mut self.modal_state
                {
                    *modal_rec_relay_input = text;
                }
            }
            Message::OpenAddContactModal => {
                self.modal_state = ModalState::add_contact(None);
            }
            Message::CloseAddContactModal => {
                self.reset_inputs();
            }
            Message::OpenImportContactModal => {
                self.modal_state = ModalState::import_contacts();
            }
            Message::CloseImportContactModal => {
                self.reset_inputs();
            }
            Message::SaveImportedContacts(imported_contacts) => {
                back_conn.send(net::Message::ImportContacts(imported_contacts));
                self.reset_inputs()
            }
            Message::FileImporterMessage(msg) => {
                if let ModalState::ImportList {
                    imported_contacts,
                    file_importer,
                } = &mut self.modal_state
                {
                    if let Some(returned_msg) = file_importer.update(msg) {
                        handle_file_importer_message(returned_msg, imported_contacts);
                    }
                }
            }
            Message::BackEndEvent(db_ev) => match db_ev {
                net::Event::GotContacts(db_contacts) => {
                    self.contacts = db_contacts;
                }
                net::Event::DatabaseSuccessEvent(kind) => match kind {
                    net::DatabaseSuccessEventKind::ContactCreated(_)
                    | net::DatabaseSuccessEventKind::ContactDeleted(_)
                    | net::DatabaseSuccessEventKind::ContactUpdated(_)
                    | net::DatabaseSuccessEventKind::ContactsImported(_) => {
                        back_conn.send(net::Message::FetchContacts);
                    }
                    _ => (),
                },
                _ => (),
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Contacts");

        let search_contact = text_input("Search", &self.search_contact_input)
            .on_input(Message::SearchContactInputChange)
            .width(SEARCH_CONTACT_WIDTH);
        let add_contact_btn = button("Add")
            .on_press(Message::OpenAddContactModal)
            .width(ADD_CONTACT_WIDTH);
        let import_btn = button("Import")
            .on_press(Message::OpenImportContactModal)
            .width(IMPORT_CONTACT_WIDTH);
        let empty = container(text("")).width(Length::Fill);
        let utils_row = row![search_contact, empty, add_contact_btn, import_btn]
            .spacing(5)
            .width(Length::Fill);
        let contact_list: Element<_> = self
            .contacts
            .iter()
            .filter(|c| contact_matches_search(c, &self.search_contact_input))
            .map(|c| ContactRow::from_db_contact(c))
            .fold(column![].spacing(5), |col, contact| {
                col.push(contact.view().map(Message::ContactRowMessage))
            })
            .into();
        let contact_list_scroller = column![ContactRow::header(), scrollable(contact_list)];
        let primary_content: Element<_> = column![title, utils_row, contact_list_scroller]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let content: Element<_> = match &self.modal_state {
            ModalState::ContactDetails {
                modal_petname_input,
                modal_pubkey_input,
                modal_rec_relay_input,
                is_edit,
            } => Modal::new(true, primary_content, move || {
                let add_relay_input = text_input_group(
                    "Contact Name",
                    "",
                    modal_petname_input,
                    None,
                    Message::ModalPetNameInputChange,
                );
                let add_pubkey_input = text_input_group(
                    "Contact PubKey",
                    "",
                    modal_pubkey_input,
                    None,
                    Message::ModalPubKeyInputChange,
                );
                let add_rec_relay_input = text_input_group(
                    "Recommended Relay",
                    "",
                    modal_rec_relay_input,
                    None,
                    Message::ModalRecRelayInputChange,
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
            } => Modal::new(true, primary_content, || {
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
            ModalState::Off => primary_content.into(),
        };
        container(content)
            .center_x()
            .center_y()
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }
}

fn contact_matches_search(contact: &DbContact, search: &str) -> bool {
    let pubkey_matches = contact
        .pubkey
        .to_string()
        .to_lowercase()
        .contains(&search.to_lowercase());
    let petname_matches = contact.petname.as_ref().map_or(false, |petname| {
        petname.to_lowercase().contains(&search.to_lowercase())
    });

    pubkey_matches || petname_matches
}

fn handle_file_importer_message(
    returned_msg: file_importer::Message,
    imported_contacts: &mut Vec<DbContact>,
) {
    if let file_importer::Message::OnChoose(path) = returned_msg {
        match json_reader::<String, UncheckedEvent>(path) {
            Ok(contact_event) => {
                if let Kind::ContactList = contact_event.kind {
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
const SEARCH_CONTACT_WIDTH: f32 = 200.0;
const ADD_CONTACT_WIDTH: f32 = 50.0;
const IMPORT_CONTACT_WIDTH: f32 = 50.0;
