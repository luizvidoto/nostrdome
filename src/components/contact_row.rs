use iced::widget::{button, row, text};
use iced::{Element, Length};
use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::db::DbContact;
use crate::utils::format_pubkey;

#[derive(Debug, Clone)]
pub enum Message {
    DeleteContact(XOnlyPublicKey),
}
#[derive(Debug, Clone)]
pub struct ContactRow {
    petname: Option<String>,
    pubkey: XOnlyPublicKey,
    relay_url: Option<String>,
}
impl ContactRow {
    pub fn from_db_contact(contact: &DbContact) -> Self {
        Self {
            petname: contact.petname.clone(),
            pubkey: contact.pubkey.clone(),
            relay_url: contact.recommended_relay.clone(),
        }
    }
    pub fn view(&self) -> Element<'static, Message> {
        row![
            text(&self.petname.to_owned().unwrap_or("".into())).width(Length::Fill),
            text(format_pubkey(&self.pubkey.to_string())).width(Length::Fill),
            text(
                &self
                    .relay_url
                    .to_owned()
                    .map(|url| url.to_string())
                    .unwrap_or("".into())
            )
            .width(Length::Fill),
            text("").width(Length::Fill),
            button("Remove")
                .on_press(Message::DeleteContact(self.pubkey.clone()))
                .width(Length::Fill)
        ]
        .into()
    }
}
