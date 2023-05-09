use chrono::{Datelike, Utc};
use iced::widget::{button, column, container, image, row, text};
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

#[derive(Debug, Clone, Copy)]
pub enum CardMode {
    Small,
    Full,
}

#[derive(Debug, Clone)]
pub struct ContactCard {
    active_contact: Option<DbContact>,
    mode: CardMode,
    pub contact: DbContact,
    profile_img_handle: Option<image::Handle>,
}

impl ContactCard {
    pub fn from_db_contact(db_contact: &DbContact) -> Self {
        let mut profile_img_handle = None;
        if let Some(profile_img_str) = db_contact.local_profile_image_str() {
            profile_img_handle = Some(image::Handle::from_path(profile_img_str));
        }
        Self {
            active_contact: None,
            mode: CardMode::Full,
            contact: db_contact.clone(),
            profile_img_handle,
        }
    }
    pub fn view(&self) -> Element<Message> {
        let mut is_active = false;

        if let Some(contact) = &self.active_contact {
            is_active = contact == &self.contact;
        }

        let pic_element: Element<_> = match &self.profile_img_handle {
            Some(handle) => image::Image::new(handle.clone()).into(),
            None => self.name_element(true),
        };
        let pic_container = container(pic_element).width(PIC_WIDTH);

        let btn_content: Element<_> = match self.mode {
            CardMode::Small => {
                let content: Element<_> = column![pic_container, self.make_notifications()].into();
                content.into()
            }
            CardMode::Full => {
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
                let card_top_row = container(
                    row![
                        text(self.contact.select_display_name()).size(24),
                        last_date_cp,
                    ]
                    .spacing(5),
                )
                .width(Length::Fill);

                let card_bottom_row = iced_lazy::responsive(|size| {
                    // --- BOTTOM ROW ---
                    let last_message_cp: Element<_> = match self.contact.last_message_content() {
                        Some(content) => {
                            let left_pixels = size.width - NOTIFICATION_COUNT_WIDTH - 5.0; //spacing;
                            let pixel_p_char = 8.0; // 8px = 1 char
                            let taker = (left_pixels / pixel_p_char).floor() as usize;
                            let content = if taker > content.len() {
                                content
                            } else {
                                let truncated =
                                    content.graphemes(true).take(taker).collect::<String>();
                                format!("{}...", &truncated)
                            };
                            container(text(&content).size(18.0))
                                .width(Length::Fill)
                                .into()
                        }
                        None => text("").into(),
                    };
                    container(
                        row![last_message_cp, self.make_notifications()]
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
            }
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
                let mut profile_img_handle = None;
                if let Some(profile_img_str) = db_contact.local_profile_image_str() {
                    profile_img_handle = Some(image::Handle::from_path(profile_img_str));
                }
                self.profile_img_handle = profile_img_handle;
                self.contact = db_contact;
            }
            Message::UpdateActiveContact(contact) => {
                self.active_contact = Some(contact.clone());
            }
            Message::ShowOnlyProfileImage => {
                self.mode = CardMode::Small;
            }
            Message::ShowFullCard => self.mode = CardMode::Full,
        }
    }

    fn make_notifications<'a>(&self) -> Element<'a, Message> {
        match self.contact.unseen_messages() {
            0 => text("").into(),
            count => container(
                button(text(count).size(16))
                    .padding([2, 5])
                    .style(style::Button::Notification),
            )
            .width(NOTIFICATION_COUNT_WIDTH)
            .align_x(alignment::Horizontal::Right)
            .into(),
        }
    }
}

const PIC_WIDTH: f32 = 50.0;
const CARD_HEIGHT: f32 = 80.0;
const NOTIFICATION_COUNT_WIDTH: f32 = 30.0;
