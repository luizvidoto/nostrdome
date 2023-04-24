use iced::widget::{button, row, text};
use iced::Length;
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::db::DbContact;
use crate::utils::format_pubkey;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    DeleteContact(XOnlyPublicKey),
    EditContact(DbContact),
}
#[derive(Debug, Clone)]
pub struct ContactRow {
    contact: DbContact,
}

impl From<ContactRow> for DbContact {
    fn from(row: ContactRow) -> Self {
        row.contact
    }
}

impl From<&ContactRow> for DbContact {
    fn from(row: &ContactRow) -> Self {
        row.contact.to_owned()
    }
}

impl ContactRow {
    pub fn from_db_contact(contact: &DbContact) -> Self {
        Self {
            contact: contact.clone(),
        }
    }
    pub fn header<M: 'static>() -> Element<'static, M> {
        row![
            text("Public Key").width(Length::Fill),
            text("Name").width(Length::Fill),
            text("Relay").width(Length::Fill),
            // text("Image").width(Length::Fill),
            text("").width(EDIT_BTN_WIDTH),
            text("").width(REMOVE_BTN_WIDTH),
        ]
        .into()
    }
    pub fn view(&self) -> Element<'static, Message> {
        row![
            text(format_pubkey(&self.contact.pubkey.to_string())).width(Length::Fill),
            text(&self.contact.petname.to_owned().unwrap_or("".into())).width(Length::Fill),
            text(
                &self
                    .contact
                    .relay_url
                    .to_owned()
                    .map(|url| url.to_string())
                    .unwrap_or("".into())
            )
            .width(Length::Fill),
            // text("Image").width(Length::Fill),
            button("Edit")
                .on_press(Message::EditContact(self.into()))
                .width(EDIT_BTN_WIDTH),
            button("Remove")
                .on_press(Message::DeleteContact(self.contact.pubkey.clone()))
                .width(REMOVE_BTN_WIDTH)
        ]
        .into()
    }
}

const EDIT_BTN_WIDTH: f32 = 50.0;
const REMOVE_BTN_WIDTH: f32 = 100.0;
