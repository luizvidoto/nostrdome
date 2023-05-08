use chrono::{Datelike, Utc};
use iced::widget::{button, column, container, row, text};
use iced::{alignment, Length};
use unicode_segmentation::UnicodeSegmentation;

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
pub struct ContactCard {
    active_contact: Option<DbContact>,
    only_profile: bool,
    pub contact: DbContact,
}

impl ContactCard {
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

        let unseen_messages: Element<_> = {
            match self.contact.unseen_messages() {
                0 => text("").into(),
                count => container(text(count))
                    .width(NOTIFICATION_COUNT_WIDTH)
                    .align_x(alignment::Horizontal::Right)
                    .into(),
            }
        };

        // let pic: Element<_> = match self.contact.get_profile_image() {
        //     Some(_image) => text("pic").into(),
        //     None => self.name_element(true),
        // };
        let pic = self.name_element(true);
        let pic_container = container(pic).width(PIC_WIDTH);

        let btn_content: Element<_> = if self.only_profile {
            column![pic_container, unseen_messages].into()
        } else {
            // --- TOP ROW ---

            let last_date_cp: Element<_> = match self.contact.last_message_date() {
                Some(date) => {
                    let now = Utc::now().naive_utc();
                    let date_format = if date.day() == now.day() {
                        "%H:%M"
                    } else {
                        "%Y-%m-%d"
                    };

                    container(text(&date.format(date_format)).size(18.0))
                        .align_x(alignment::Horizontal::Right)
                        .width(Length::Fill)
                        .into()
                }
                None => text("").into(),
            };
            let card_top_row = container(row![self.name_element(false), last_date_cp,].spacing(5))
                .width(Length::Fill);

            let card_bottom_row = iced_lazy::responsive(|size| {
                let unseen_messages: Element<_> = {
                    match self.contact.unseen_messages() {
                        0 => text("").into(),
                        count => container(text(count))
                            .width(NOTIFICATION_COUNT_WIDTH)
                            .align_x(alignment::Horizontal::Right)
                            .into(),
                    }
                };
                // --- BOTTOM ROW ---
                let last_message_cp: Element<_> = match self.contact.last_message_content() {
                    Some(content) => {
                        let left_pixels = size.width - NOTIFICATION_COUNT_WIDTH - 5.0; //spacing;
                        let pixel_p_char = 8.0; // 8px = 1 char
                        let taker = (left_pixels / pixel_p_char).floor() as usize;
                        let content = if taker > content.len() {
                            content
                        } else {
                            let truncated = content.graphemes(true).take(taker).collect::<String>();
                            format!("{}...", &truncated)
                        };
                        container(text(&content).size(18.0))
                            .width(Length::Fill)
                            .into()
                    }
                    None => text("").into(),
                };
                container(
                    row![last_message_cp, unseen_messages,]
                        .align_items(alignment::Alignment::Center)
                        .spacing(5),
                )
                .width(Length::Fill)
                .into()
            });

            let expanded_card = column![card_top_row, card_bottom_row].width(Length::Fill);

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
const NOTIFICATION_COUNT_WIDTH: f32 = 30.0;
