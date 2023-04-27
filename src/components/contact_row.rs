use iced::widget::{button, row, text};
use iced::Length;

use crate::db::DbContact;
use crate::icon::{delete_icon, edit_icon};
use crate::utils::format_pubkey;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    DeleteContact(DbContact),
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
            text(format_pubkey(&self.contact.pubkey().to_string())).width(Length::Fill),
            text(&self.contact.get_petname().unwrap_or("".into())).width(Length::Fill),
            text(
                &self
                    .contact
                    .get_relay_url()
                    .map(|url| url.to_string())
                    .unwrap_or("".into())
            )
            .width(Length::Fill),
            // text("Image").width(Length::Fill),
            button(edit_icon().size(16)).on_press(Message::EditContact(self.into())),
            button(delete_icon().size(16)).on_press(Message::DeleteContact(self.contact.clone())) // .style(style::Button::Danger)
        ]
        .into()
    }
}

const EDIT_BTN_WIDTH: f32 = 50.0;
const REMOVE_BTN_WIDTH: f32 = 100.0;
