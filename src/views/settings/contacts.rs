use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Element, Length};
use iced_aw::{Card, Modal};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::components::text_input_group::text_input_group;
use crate::components::{contact_row, file_importer, ContactRow, FileImporter};
use crate::net::BackEndConnection;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    SubmitNewContact,
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
}

#[derive(Debug, Clone)]
enum ModalState {
    AddContact,
    ImportList,
    Off,
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    file_importer: FileImporter,
    modal_state: ModalState,
    modal_petname_input: String,
    modal_pubkey_input: String,
    modal_rec_relay_input: String,
}
impl State {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            file_importer: FileImporter::new("/path/to/contacts.json")
                .file_filter("JSON File", &["json"]),
            modal_state: ModalState::Off,
            modal_petname_input: "".into(),
            modal_pubkey_input: "".into(),
            modal_rec_relay_input: "".into(),
        }
    }

    pub fn reset_inputs(&mut self) {
        self.modal_state = ModalState::Off;
        self.modal_petname_input = "".into();
        self.modal_pubkey_input = "".into();
        self.modal_rec_relay_input = "".into();
        self.file_importer.reset_inputs();
    }

    pub fn update(&mut self, message: Message, back_conn: &mut BackEndConnection) {
        match message {
            Message::ContactRowMessage(ct_msg) => match ct_msg {
                contact_row::Message::DeleteContact(pubkey) => {
                    back_conn.send(net::Message::DeleteContact(pubkey))
                }
            },
            Message::DeleteContact(pubkey) => {
                back_conn.send(net::Message::DeleteContact(pubkey));
            }
            Message::SubmitNewContact => match DbContact::from_str(&self.modal_pubkey_input) {
                Ok(mut db_contact) => {
                    db_contact = db_contact
                        .petname(&self.modal_petname_input)
                        .recommended_relay(&self.modal_rec_relay_input);
                    back_conn.send(net::Message::AddContact(db_contact));
                    self.reset_inputs();
                }
                Err(_e) => {
                    tracing::error!("Every contact must have a valid pubkey.");
                }
            },
            Message::ModalPetNameInputChange(text_change) => {
                self.modal_petname_input = text_change;
            }
            Message::ModalPubKeyInputChange(text_change) => {
                self.modal_pubkey_input = text_change;
            }
            Message::ModalRecRelayInputChange(text_change) => {
                self.modal_rec_relay_input = text_change;
            }
            Message::OpenAddContactModal => {
                self.modal_state = ModalState::AddContact;
            }
            Message::CloseAddContactModal => {
                self.reset_inputs();
            }
            Message::OpenImportContactModal => {
                self.modal_state = ModalState::ImportList;
            }
            Message::CloseImportContactModal => {
                self.reset_inputs();
            }
            Message::FileImporterMessage(msg) => {
                if let Some(returned_msg) = self.file_importer.update(msg) {
                    if let file_importer::Message::OnChoose(path) = returned_msg {
                        println!("path: {}", path);
                    }
                }
            }
            Message::BackEndEvent(db_ev) => match db_ev {
                net::Event::GotContacts(db_contacts) => {
                    self.contacts = db_contacts;
                }
                net::Event::DatabaseSuccessEvent(kind) => match kind {
                    net::DatabaseSuccessEventKind::ContactCreated
                    | net::DatabaseSuccessEventKind::ContactDeleted
                    | net::DatabaseSuccessEventKind::ContactUpdated => {
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
        let table_header = row![
            text("petname").width(Length::Fill),
            text("pubkey").width(Length::Fill),
            text("recommended_relay").width(Length::Fill),
            text("profile_image").width(Length::Fill),
            container(text("")).width(Length::Fill),
        ];
        let add_contact_btn = button("Add").on_press(Message::OpenAddContactModal);
        let import_btn = button("Import").on_press(Message::OpenImportContactModal);
        let btn_row = row![add_contact_btn, import_btn]
            .spacing(5)
            .width(Length::Fill);
        let contact_list: Element<_> = self
            .contacts
            .iter()
            .map(|c| ContactRow::from_db_contact(c))
            .fold(column![].spacing(5), |col: Column<'_, Message>, contact| {
                col.push(contact.view().map(Message::ContactRowMessage))
            })
            .into();
        let contact_list_scroller = column![table_header, scrollable(contact_list)];
        let primary_content: Element<_> = column![title, btn_row, contact_list_scroller]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let content: Element<_> = match self.modal_state {
            ModalState::AddContact => {
                Modal::new(true, primary_content, || {
                    let add_relay_input = text_input_group(
                        "Contact Name",
                        "",
                        &self.modal_petname_input,
                        None,
                        Message::ModalPetNameInputChange,
                    );
                    let add_pubkey_input = text_input_group(
                        "Contact PubKey",
                        "",
                        &self.modal_pubkey_input,
                        None,
                        Message::ModalPubKeyInputChange,
                    );
                    let add_rec_relay_input = text_input_group(
                        "Recommended Relay",
                        "",
                        &self.modal_rec_relay_input,
                        None,
                        Message::ModalRecRelayInputChange,
                    );
                    let modal_body: Element<_> =
                        column![add_relay_input, add_pubkey_input, add_rec_relay_input]
                            .spacing(4)
                            .into();
                    Card::new(text("Add Contact"), modal_body)
                        .foot(
                            row![
                                button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::CloseAddContactModal),
                                button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::SubmitNewContact)
                            ]
                            .spacing(10)
                            .padding(5)
                            .width(Length::Fill),
                        )
                        .max_width(ADD_CONTACT_MODAL_WIDTH)
                        //.width(Length::Shrink)
                        .on_close(Message::CloseAddContactModal)
                        .into()
                })
                .backdrop(Message::CloseAddContactModal)
                .on_esc(Message::CloseAddContactModal)
                .into()
            }
            ModalState::ImportList => {
                Modal::new(true, primary_content, || {
                    let modal_body: Element<_> =
                        column![self.file_importer.view().map(Message::FileImporterMessage)]
                            .spacing(4)
                            .into();
                    Card::new(text("Import Contacts"), modal_body)
                        .foot(
                            row![
                                button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::CloseImportContactModal),
                                button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::SubmitNewContact)
                            ]
                            .spacing(10)
                            .padding(5)
                            .width(Length::Fill),
                        )
                        .max_width(IMPORT_MODAL_WIDTH)
                        //.width(Length::Shrink)
                        .on_close(Message::CloseImportContactModal)
                        .into()
                })
                .backdrop(Message::CloseImportContactModal)
                .on_esc(Message::CloseImportContactModal)
                .into()
            }
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

const ADD_CONTACT_MODAL_WIDTH: f32 = 400.0;
const IMPORT_MODAL_WIDTH: f32 = 400.0;
