use iced::widget::{button, row, text};
use iced::{Element, Length};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::db::{ContactStatus, DbContact};
use crate::utils::format_pubkey;

#[derive(Debug, Clone)]
pub enum Message {
    DeleteContact(XOnlyPublicKey),
    EditContact(DbContact),
}
#[derive(Debug, Clone)]
pub struct ContactRow {
    petname: Option<String>,
    pubkey: XOnlyPublicKey,
    relay_url: Option<String>,
    status: ContactStatus,
}

impl From<ContactRow> for DbContact {
    fn from(row: ContactRow) -> Self {
        (&row).into()
    }
}

impl From<&ContactRow> for DbContact {
    fn from(row: &ContactRow) -> Self {
        DbContact {
            pubkey: row.pubkey.clone(),
            relay_url: row.relay_url.clone(),
            petname: row.petname.clone(),
            profile_image: None,
            status: row.status,
        }
    }
}

impl ContactRow {
    pub fn from_db_contact(contact: &DbContact) -> Self {
        Self {
            petname: contact.petname.clone(),
            pubkey: contact.pubkey.clone(),
            relay_url: contact.relay_url.clone(),
            status: contact.status,
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
            text(format_pubkey(&self.pubkey.to_string())).width(Length::Fill),
            text(&self.petname.to_owned().unwrap_or("".into())).width(Length::Fill),
            text(
                &self
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
                .on_press(Message::DeleteContact(self.pubkey.clone()))
                .width(REMOVE_BTN_WIDTH)
        ]
        .into()
    }
}

const EDIT_BTN_WIDTH: f32 = 50.0;
const REMOVE_BTN_WIDTH: f32 = 100.0;
