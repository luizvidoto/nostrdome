use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Element, Length};
use iced_aw::{Card, Modal};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::components::text_input_group::text_input_group;
use crate::components::{contact_row, ContactRow};
use crate::net::BackEndConnection;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    CloseAddContactModal,
    SubmitNewContact,
    OpenAddContactModal,
    BackEndEvent(net::Event),
    ModalPetNameInputChange(String),
    ModalPubKeyInputChange(String),
    ModalRecRelayInputChange(String),
    DeleteContact(XOnlyPublicKey),
    ContactRowMessage(contact_row::Message),
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    show_modal: bool,
    modal_petname_input: String,
    modal_pubkey_input: String,
    modal_rec_relay_input: String,
}
impl State {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            show_modal: false,
            modal_petname_input: "".into(),
            modal_pubkey_input: "".into(),
            modal_rec_relay_input: "".into(),
        }
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
                    self.show_modal = false;
                    self.modal_petname_input = "".into();
                    self.modal_pubkey_input = "".into();
                    self.modal_rec_relay_input = "".into();
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
                self.show_modal = true;
            }
            Message::CloseAddContactModal => {
                self.show_modal = false;
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
        let import_list_btn = button("Import List").on_press(Message::OpenAddContactModal);
        let import_from_relay_btn = button("Import Relay").on_press(Message::OpenAddContactModal);
        let btn_row = row![add_contact_btn, import_list_btn, import_from_relay_btn,]
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
        let content: Element<_> = column![title, btn_row, contact_list_scroller]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        Modal::new(self.show_modal, content, || {
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
                .max_width(300.0)
                //.width(Length::Shrink)
                .on_close(Message::CloseAddContactModal)
                .into()
        })
        .backdrop(Message::CloseAddContactModal)
        .on_esc(Message::CloseAddContactModal)
        .into()
    }
}
