use iced::widget::{button, container, row, text};
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
    // pub fn update(&mut self, db_contact: &DbContact) {
    //     self.contact.update_base_from_other(db_contact);
    // }
    pub fn header<M: 'static>() -> Element<'static, M> {
        row![
            container(text("Public Key")).width(Length::Fixed(PUBKEY_CELL_WIDTH)),
            container(text("Name"))
                .width(Length::Fixed(NAME_CELL_WIDTH_MIN))
                .max_width(NAME_CELL_WIDTH_MAX),
            container(text("Relay"))
                .align_x(iced::alignment::Horizontal::Left)
                .width(Length::Fill),
            container(text("")).width(Length::Fixed(EDIT_BTN_WIDTH)),
            container(text("")).width(Length::Fixed(REMOVE_BTN_WIDTH)),
        ]
        .spacing(2)
        .into()
    }
    pub fn view(&self) -> Element<'static, Message> {
        row![
            container(text(format_pubkey(&self.contact.pubkey().to_string())))
                .width(Length::Fixed(PUBKEY_CELL_WIDTH)),
            container(text(&self.contact.get_petname().unwrap_or("".into())))
                .width(Length::Fixed(NAME_CELL_WIDTH_MIN))
                .max_width(NAME_CELL_WIDTH_MAX),
            container(text(
                &self
                    .contact
                    .get_relay_url()
                    .map(|url| url.to_string())
                    .unwrap_or("".into())
            ))
            .width(Length::Fill),
            container(button(edit_icon().size(16)).on_press(Message::EditContact(self.into())))
                .width(Length::Fixed(EDIT_BTN_WIDTH)),
            container(
                button(delete_icon().size(16))
                    .on_press(Message::DeleteContact(self.contact.clone()))
            )
            .width(Length::Fixed(REMOVE_BTN_WIDTH)) // .style(style::Button::Danger)
        ]
        .spacing(2)
        .into()
    }
}

const EDIT_BTN_WIDTH: f32 = 30.0;
const REMOVE_BTN_WIDTH: f32 = 30.0;
const PUBKEY_CELL_WIDTH: f32 = 100.0;
const NAME_CELL_WIDTH_MIN: f32 = 100.0;
const NAME_CELL_WIDTH_MAX: f32 = 200.0;
