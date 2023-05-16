use chrono::{Datelike, Utc};
use iced::widget::{button, column, container, image, row, text};
use iced::{alignment, Length};
use unicode_segmentation::UnicodeSegmentation;

use crate::consts::YMD_FORMAT;
use crate::db::DbContact;
use crate::net::{BackEndConnection, ImageSize};
use crate::style;
use crate::utils::from_naive_utc_to_local;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub struct MessageWrapper {
    pub message: Message,
    pub from: i32,
}
impl MessageWrapper {
    pub fn new(from: i32, message: Message) -> Self {
        Self { from, message }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ContactUpdated(DbContact),
    ContactPress(DbContact),
    TurnOffActive,
    TurnOnActive,
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
    pub id: i32,
    card_active: bool,
    mode: CardMode,
    pub contact: DbContact,
    profile_img_handle: image::Handle,
}

impl ContactCard {
    pub fn from_db_contact(id: i32, db_contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        let size = ImageSize::Small;
        let profile_img_handle = db_contact.profile_image(size, conn);
        Self {
            id,
            card_active: false,
            mode: CardMode::Full,
            contact: db_contact.clone(),
            profile_img_handle,
        }
    }
    pub fn view(&self) -> Element<MessageWrapper> {
        let size = ImageSize::Small;

        let (width, height) = size.get_width_height().unwrap();
        let pic_container = container(image(self.profile_img_handle.to_owned()))
            .width(width as f32)
            .height(height as f32);

        let btn_content: Element<_> = match self.mode {
            CardMode::Small => {
                let content: Element<_> = column![pic_container, self.make_notifications()].into();
                content.into()
            }
            CardMode::Full => {
                // --- TOP ROW ---
                let last_date_cp = self.make_last_date();
                let card_top_row = container(
                    row![text(self.contact.select_name()).size(24), last_date_cp,].spacing(5),
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
            .on_press(MessageWrapper::new(
                self.id,
                Message::ContactPress(self.contact.clone()),
            ))
            .style(if self.card_active {
                style::Button::ActiveContactCard
            } else {
                style::Button::ContactCard
            })
            .into()
    }

    fn make_last_date<'a>(&'a self) -> Element<'a, MessageWrapper> {
        match self.contact.last_message_date() {
            Some(date) => {
                let local_day = from_naive_utc_to_local(date);
                let local_now = from_naive_utc_to_local(Utc::now().naive_utc());
                let date_format = if local_day.day() == local_now.day() {
                    "%H:%M"
                } else {
                    // TODO: get local system language
                    // settings menu to change it
                    YMD_FORMAT
                };

                container(text(&local_day.format(date_format)).size(18.0))
                    .align_x(alignment::Horizontal::Right)
                    .width(Length::Fill)
                    .into()
            }
            None => text("").into(),
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            // Message::ContactProfilePictureUpdated(db_contact)
            Message::ContactUpdated(db_contact) => {
                self.profile_img_handle = db_contact.profile_image(ImageSize::Small, conn);
                self.contact = db_contact;
            }
            Message::ContactPress(_) => (),
            Message::TurnOffActive => {
                self.card_active = false;
            }
            Message::TurnOnActive => {
                self.card_active = true;
            }
            Message::ShowOnlyProfileImage => {
                self.mode = CardMode::Small;
            }
            Message::ShowFullCard => self.mode = CardMode::Full,
        }
    }

    fn make_notifications<'a>(&self) -> Element<'a, MessageWrapper> {
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

const CARD_HEIGHT: f32 = 80.0;
const NOTIFICATION_COUNT_WIDTH: f32 = 30.0;
