use chrono::{Datelike, NaiveDateTime, Utc};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image, row, text};
use iced::{alignment, Length};
use unicode_segmentation::UnicodeSegmentation;

use crate::consts::YMD_FORMAT;
use crate::db::{DbContact, ImageDownloaded};
use crate::error::BackendClosed;
use crate::net::{self, BackEndConnection, ImageSize};
use crate::style;
use crate::types::ChatMessage;
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
    ContactPress(i32),
}

pub enum CardMode {
    Small,
    Full,
}

#[derive(Debug, Clone)]
pub struct ChatInfo {
    pub unseen_messages: i64,
    pub last_message: String,
    pub last_message_time: Option<NaiveDateTime>,
}
impl ChatInfo {
    fn should_update(&self, new_date: Option<&NaiveDateTime>) -> bool {
        if let Some(new_date) = new_date {
            if let Some(last_time) = self.last_message_time.as_ref() {
                return new_date > last_time;
            }
        }
        true
    }
    pub fn update(&mut self, new_info: ChatInfo) {
        if self.should_update(new_info.last_message_time.as_ref()) {
            *self = new_info;
        }
    }
    pub fn update_headers(&mut self, msg: &ChatMessage) {
        if self.should_update(msg.display_time()) {
            self.last_message = msg.content().to_owned();
            self.last_message_time = msg.display_time().cloned();
        }
    }
    fn add(&mut self) {
        self.unseen_messages = (self.unseen_messages + 1).min(100);
    }
}
impl Default for ChatInfo {
    fn default() -> Self {
        Self {
            unseen_messages: 0,
            last_message: "".into(),
            last_message_time: None,
        }
    }
}

pub struct ChatContact {
    pub id: i32,
    mode: CardMode,
    pub contact: DbContact,
    profile_img_handle: image::Handle,
    chat_info: ChatInfo,
}

impl ChatContact {
    pub fn new(
        id: i32,
        db_contact: &DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        conn.send(net::ToBackend::FetchChatInfo(db_contact.clone()))?;
        let size = ImageSize::Small;
        let profile_img_handle = db_contact.profile_image(size, conn)?;
        Ok(Self {
            id,
            mode: CardMode::Full,
            contact: db_contact.clone(),
            profile_img_handle,
            chat_info: ChatInfo::default(),
        })
    }
    pub fn view(&self, active_id: Option<i32>) -> Element<MessageWrapper> {
        let size = ImageSize::Small;
        let card_active = active_id.map(|id| id == self.id);
        let (width, height) = size.get_width_height().unwrap();
        let pic_container = container(image(self.profile_img_handle.to_owned()))
            .width(width as f32)
            .height(height as f32);

        let btn_content: Element<_> = match self.mode {
            CardMode::Small => column![pic_container, self.make_notifications()].into(),
            CardMode::Full => {
                // --- TOP ROW ---
                let last_date_cp = self.make_last_date();
                let card_top_row = container(
                    row![text(self.contact.select_name()).size(24), last_date_cp,].spacing(5),
                )
                .width(Length::Fill);

                let card_bottom_row = iced_lazy::responsive(|size| {
                    // --- BOTTOM ROW ---
                    let content = &self.chat_info.last_message;
                    let left_pixels = size.width - NOTIFICATION_COUNT_WIDTH - 5.0; //spacing;
                    let pixel_p_char = 8.0; // 8px = 1 char
                    let taker = (left_pixels / pixel_p_char).floor() as usize;
                    let content = if taker > content.len() {
                        content.to_owned()
                    } else {
                        let truncated = content.graphemes(true).take(taker).collect::<String>();
                        format!("{}...", &truncated)
                    };
                    let last_message_cp = container(text(&content).size(18.0)).width(Length::Fill);

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

        let mut card_style = style::Button::ContactCard;
        if let Some(card_active) = card_active {
            if card_active {
                card_style = style::Button::ActiveContactCard;
            }
        }

        button(btn_content)
            .width(Length::Fill)
            .height(CARD_HEIGHT)
            .on_press(MessageWrapper::new(self.id, Message::ContactPress(self.id)))
            .style(card_style)
            .into()
    }

    fn make_last_date(&self) -> Element<'_, MessageWrapper> {
        let Some(date) = &self.chat_info.last_message_time else {
            return text("").into();
        };

        let local_day = from_naive_utc_to_local(*date);
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

    pub fn new_message(&mut self, chat_message: ChatMessage) {
        self.update_headers(chat_message);
        self.chat_info.add();
    }
    pub fn update_headers(&mut self, chat_message: ChatMessage) {
        self.chat_info.update_headers(&chat_message);
    }
    pub fn reset_unseen(&mut self) {
        self.chat_info.unseen_messages = 0;
    }
    pub fn update_chat_info(&mut self, new_info: ChatInfo) {
        self.chat_info.update(new_info);
    }
    pub fn update_image(&mut self, image: ImageDownloaded) {
        let path = image.sized_image(ImageSize::Small);
        self.profile_img_handle = Handle::from_path(path);
    }
    pub fn update_contact(
        &mut self,
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        self.profile_img_handle = db_contact.profile_image(ImageSize::Small, conn)?;
        self.contact = db_contact;
        Ok(())
    }
    pub fn small_mode(&mut self) {
        self.mode = CardMode::Small;
    }
    pub fn big_mode(&mut self) {
        self.mode = CardMode::Full;
    }

    fn make_notifications<'a>(&self) -> Element<'a, MessageWrapper> {
        let count_txt = match self.chat_info.unseen_messages {
            0 => return text("").into(),
            1..=99 => self.chat_info.unseen_messages.to_string(),
            _ => "99+".into(),
        };

        container(
            button(text(count_txt).size(16))
                .padding([2, 4])
                .style(style::Button::Notification),
        )
        .width(NOTIFICATION_COUNT_WIDTH)
        .align_x(alignment::Horizontal::Right)
        .into()
    }

    pub(crate) fn height(&self) -> f32 {
        CARD_HEIGHT
    }

    pub(crate) fn last_message_date(&self) -> Option<NaiveDateTime> {
        self.chat_info.last_message_time
    }
}

pub(crate) const CARD_HEIGHT: f32 = 80.0;
const NOTIFICATION_COUNT_WIDTH: f32 = 30.0;
