use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Element, Length};
use iced_aw::{Card, Modal};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::components::text_input_group::text_input_group;
use crate::net::database::DbConnection;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    CloseAddContactModal,
    SubmitNewContact,
    OpenAddContactModal,
    DbEvent(net::database::Event),
    ModalPetNameInputChange(String),
    ModalPubKeyInputChange(String),
    ModalRecRelayInputChange(String),
    DeleteContact(XOnlyPublicKey),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Loaded {
        contacts: Vec<DbContact>,
        show_modal: bool,
        modal_petname_input: String,
        modal_pubkey_input: String,
        modal_rec_relay_input: String,
    },
}
impl State {
    pub fn loading(db_conn: &mut DbConnection) -> Self {
        if let Err(e) = db_conn.send(net::database::Message::FetchContacts) {
            tracing::error!("{}", e);
        };
        Self::Loading
    }
    pub fn loaded(contacts: &[DbContact]) -> Self {
        Self::Loaded {
            contacts: contacts.to_vec(),
            show_modal: false,
            modal_petname_input: "".into(),
            modal_pubkey_input: "".into(),
            modal_rec_relay_input: "".into(),
        }
    }

    pub fn update(&mut self, message: Message, db_conn: &mut DbConnection) {
        match self {
            Self::Loading => {
                if let Message::DbEvent(db_ev) = message {
                    if let net::database::Event::GotContacts(db_contacts) = db_ev {
                        *self = Self::loaded(&db_contacts);
                    }
                }
            }
            Self::Loaded {
                contacts,
                show_modal,
                modal_petname_input,
                modal_pubkey_input,
                modal_rec_relay_input,
            } => match message {
                Message::DeleteContact(pubkey) => {
                    if let Err(e) = db_conn.send(net::database::Message::DeleteContact(pubkey)) {
                        tracing::error!("{}", e);
                    }
                }
                Message::SubmitNewContact => match DbContact::from_str(modal_pubkey_input) {
                    Ok(mut db_contact) => {
                        db_contact = db_contact
                            .petname(modal_petname_input)
                            .recommended_relay(modal_rec_relay_input);
                        if let Err(e) = db_conn.send(net::database::Message::AddContact(db_contact))
                        {
                            tracing::error!("{}", e);
                        }
                        *show_modal = false;
                        *modal_petname_input = "".into();
                        *modal_pubkey_input = "".into();
                        *modal_rec_relay_input = "".into();
                    }
                    Err(e) => {
                        tracing::error!("Every contact must have a valid pubkey.");
                    }
                },
                Message::ModalPetNameInputChange(text_change) => {
                    *modal_petname_input = text_change;
                }
                Message::ModalPubKeyInputChange(text_change) => {
                    *modal_pubkey_input = text_change;
                }
                Message::ModalRecRelayInputChange(text_change) => {
                    *modal_rec_relay_input = text_change;
                }
                Message::OpenAddContactModal => {
                    *show_modal = true;
                }
                Message::CloseAddContactModal => {
                    *show_modal = false;
                }
                Message::DbEvent(db_ev) => match db_ev {
                    net::database::Event::GotContacts(db_contacts) => {
                        *contacts = db_contacts;
                    }
                    net::database::Event::DatabaseSuccessEvent(kind) => match kind {
                        net::database::DatabaseSuccessEventKind::ContactCreated
                        | net::database::DatabaseSuccessEventKind::ContactDeleted
                        | net::database::DatabaseSuccessEventKind::ContactUpdated => {
                            if let Err(e) = db_conn.send(net::database::Message::FetchContacts) {
                                tracing::error!("{}", e);
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                },
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
        match self {
            State::Loading => title.into(),
            State::Loaded {
                contacts,
                show_modal,
                modal_petname_input,
                modal_pubkey_input,
                modal_rec_relay_input,
            } => {
                let contact_list: Element<_> = contacts
                    .iter()
                    .fold(column![].spacing(5), |col: Column<'_, Message>, contact| {
                        col.push(row![
                            text(&contact.petname.to_owned().unwrap_or("".into()))
                                .width(Length::Fill),
                            text(&contact.pubkey).width(Length::Fill),
                            text(&contact.recommended_relay.to_owned().unwrap_or("".into()))
                                .width(Length::Fill),
                            text(&contact.profile_image.to_owned().unwrap_or("".into()))
                                .width(Length::Fill),
                            button("Remove")
                                .on_press(Message::DeleteContact(contact.pubkey.clone()))
                                .width(Length::Fill)
                        ])
                    })
                    .into();
                let contact_list_scroller = column![table_header, scrollable(contact_list)];
                let content: Element<_> = column![title, add_contact_btn, contact_list_scroller]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
                Modal::new(*show_modal, content, || {
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
    }
}
