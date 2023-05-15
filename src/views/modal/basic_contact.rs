use std::fmt::Debug;

use crate::components::text_input_group::TextInputGroup;
use crate::db::{DbContact, DbContactError};
use crate::net::{self, BackEndConnection};
use iced::alignment;
use iced::widget::{button, column, row, text};
use iced::Length;
use iced_aw::{Card, Modal};

use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    PetNameInputChange(String),
    PubKeyInputChange(String),
    RecRelayInputChange(String),
    SubmitContact,
    CloseModal,
    UnderlayMessage(M),
}
#[derive(Debug, Clone)]
pub struct ContactDetails {
    db_contact: Option<DbContact>,
    modal_petname_input: String,
    modal_pubkey_input: String,
    modal_rec_relay_input: String,
    is_edit: bool,
    is_pub_invalid: bool,
    is_relay_invalid: bool,
}
impl ContactDetails {
    pub(crate) fn new(db_contact: Option<DbContact>) -> Self {
        let (petname, pubkey, rec_relay, is_edit) = match &db_contact {
            Some(c) => (
                c.get_petname().unwrap_or_else(|| "".into()),
                c.pubkey().to_string(),
                c.get_relay_url()
                    .map(|url| url.to_string())
                    .unwrap_or("".into()),
                true,
            ),
            None => ("".into(), "".into(), "".into(), false),
        };

        Self {
            db_contact,
            modal_petname_input: petname,
            modal_pubkey_input: pubkey,
            modal_rec_relay_input: rec_relay,
            is_edit,
            is_pub_invalid: false,
            is_relay_invalid: false,
        }
    }
    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, move || {
            let add_relay_input = TextInputGroup::new(
                "Contact Name",
                &self.modal_petname_input,
                CMessage::PetNameInputChange,
            )
            .placeholder("");

            let mut add_pubkey_input = TextInputGroup::new(
                "Contact PubKey",
                &self.modal_pubkey_input,
                CMessage::PubKeyInputChange,
            )
            .placeholder("");

            if self.is_pub_invalid {
                add_pubkey_input = add_pubkey_input.invalid("Invalid Public Key");
            }

            if self.is_edit {
                add_pubkey_input = add_pubkey_input.disabled();
            }

            let mut add_rec_relay_input = TextInputGroup::new(
                "Recommended Relay",
                &self.modal_rec_relay_input,
                CMessage::RecRelayInputChange,
            )
            .placeholder("ws://my-relay.com");

            if self.is_relay_invalid {
                add_rec_relay_input = add_rec_relay_input.invalid("Invalid Relay URL");
            }

            let modal_body: Element<_> = column![
                add_relay_input.build(),
                add_pubkey_input.build(),
                add_rec_relay_input.build()
            ]
            .spacing(4)
            .into();
            let title = match self.is_edit {
                true => text("Edit Contact"),
                false => text("Add Contact"),
            };
            Card::new(title, modal_body)
                .foot(
                    row![
                        button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center),)
                            .width(Length::Fill)
                            .on_press(CMessage::CloseModal),
                        button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                            .width(Length::Fill)
                            .on_press(CMessage::SubmitContact)
                    ]
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill),
                )
                .max_width(MODAL_WIDTH)
                .on_close(CMessage::CloseModal)
                .into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }

    /// Returns true if modal is to be closed
    pub(crate) fn update<'a, M: 'a + Clone + Debug>(
        &mut self,
        message: CMessage<M>,
        conn: &mut BackEndConnection,
    ) -> bool {
        match message {
            CMessage::PetNameInputChange(text) => {
                self.modal_petname_input = text;
            }
            CMessage::PubKeyInputChange(text) => {
                self.modal_pubkey_input = text;
                self.is_pub_invalid = false;
            }
            CMessage::RecRelayInputChange(text) => {
                self.modal_rec_relay_input = text;
                self.is_relay_invalid = false;
            }
            CMessage::SubmitContact => return self.handle_submit_contact(conn),
            CMessage::CloseModal => {
                return true;
            }
            CMessage::UnderlayMessage(_) => (),
        }
        false
    }

    pub(crate) fn handle_submit_contact(&mut self, conn: &mut BackEndConnection) -> bool {
        let submit_result = match &self.db_contact {
            Some(db_contact) => DbContact::edit_contact(
                db_contact.to_owned(),
                &self.modal_petname_input,
                &self.modal_rec_relay_input,
            ),
            None => DbContact::new_from_submit(
                &self.modal_pubkey_input,
                &self.modal_petname_input,
                &self.modal_rec_relay_input,
            ),
        };

        match submit_result {
            Ok(db_contact) => {
                match self.is_edit {
                    true => conn.send(net::Message::UpdateContact(db_contact)),
                    false => conn.send(net::Message::AddContact(db_contact)),
                }

                // *self = Self::Off;
                return true;
            }
            Err(e) => {
                tracing::error!("Error: {:?}", e);
                match e {
                    DbContactError::InvalidPublicKey => {
                        self.is_pub_invalid = true;
                    }
                    DbContactError::InvalidRelayUrl(_) => {
                        self.is_relay_invalid = true;
                    }
                    _ => (),
                }
            }
        }
        false
    }
}

const MODAL_WIDTH: f32 = 400.0;
