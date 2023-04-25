use iced::widget::{button, column, container, row, text};
use iced::{alignment, Length};

use crate::db::DbContact;
use crate::style;
use crate::utils::format_pubkey;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    ContactUpdated(DbContact),
    UpdateActiveContact(DbContact),
    ShowOnlyProfileImage,
    ShowFullCard,
}

#[derive(Debug, Clone)]
pub struct State {
    active_contact: Option<DbContact>,
    only_profile: bool,
    pub contact: DbContact,
}

impl State {
    pub fn from_db_contact(db_contact: &DbContact) -> Self {
        Self {
            active_contact: None,
            only_profile: false,
            contact: db_contact.clone(),
        }
    }
    pub fn view(&self) -> Element<Message> {
        let mut is_active = false;

        if let Some(contact) = &self.active_contact {
            is_active = contact == &self.contact;
        }

        let unseen_messages: Element<_> = match self.contact.unseen_messages() {
            0 => text("").into(),
            msg => container(text(format!("{}", msg)))
                .align_x(alignment::Horizontal::Right)
                .width(Length::Fill)
                .into(),
        };

        let pic: Element<_> = match self.contact.get_profile_image() {
            Some(_image) => text("pic").into(),
            None => self.name_element(true),
        };
        let pic_container = container(pic).width(PIC_WIDTH);

        let btn_content: Element<_> = if self.only_profile {
            column![pic_container, unseen_messages].into()
        } else {
            let (last_message, last_date): (Element<_>, Element<_>) =
                match self.contact.last_message_pair() {
                    (Some(content), Some(date)) => (
                        text(&content).size(18.0).into(),
                        container(text(&date.format("%Y-%m-%d")).size(20.0))
                            .align_x(alignment::Horizontal::Right)
                            .width(Length::Fill)
                            .into(),
                    ),
                    _ => (text("").into(), text("").into()),
                };
            let expanded_card = column![
                container(row![self.name_element(false), last_date,].spacing(5))
                    .width(Length::Fill),
                container(row![last_message, unseen_messages,].spacing(5)).width(Length::Fill)
            ]
            .width(Length::Fill);
            row![pic_container, expanded_card,]
                .width(Length::Fill)
                .spacing(2)
                .into()
        };

        button(btn_content)
            .width(Length::Fill)
            .height(CARD_HEIGHT)
            .on_press(Message::UpdateActiveContact(self.contact.clone()))
            .style(if is_active {
                style::Button::ActiveContactCard
            } else {
                style::Button::ContactCard
            })
            .into()
    }

    fn name_element(&self, is_pic: bool) -> Element<'static, Message> {
        let pub_string = self.contact.pubkey().to_string();
        let formatted_pubstring = format_pubkey(&pub_string);
        let extracted_name = if is_pic {
            &pub_string[0..2]
        } else {
            &formatted_pubstring
        };

        match self.contact.get_petname() {
            Some(name) => {
                if is_pic {
                    text(&name[0..2]).into()
                } else {
                    text(name).into()
                }
            }
            None => text(format!("{}", extracted_name)).into(),
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ContactUpdated(db_contact) => {
                self.contact = db_contact;
            }
            Message::UpdateActiveContact(contact) => {
                self.active_contact = Some(contact.clone());
            }
            Message::ShowOnlyProfileImage => {
                self.only_profile = true;
            }
            Message::ShowFullCard => {
                self.only_profile = false;
            }
        }
    }
}

const PIC_WIDTH: f32 = 50.0;
const CARD_HEIGHT: f32 = 80.0;
