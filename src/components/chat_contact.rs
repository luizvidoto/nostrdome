use chrono::{Datelike, NaiveDateTime, Utc};
use iced::widget::{button, column, container, image, row, text};
use iced::{alignment, Length};
use unicode_segmentation::UnicodeSegmentation;

use crate::consts::YMD_FORMAT;
use crate::db::DbContact;
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
    ContactUpdated(DbContact),
    ContactPress(i32),
    ShowOnlyProfileImage,
    ShowFullCard,
    GotChatInfo(Option<ChatInfo>),
    AddUnseenCount,
    NewMessage(ChatMessage),
    UpdatedMetadata(DbContact),
    ResetUnseenCount,
}

#[derive(Debug, Clone, Copy)]
pub enum CardMode {
    Small,
    Full,
}

#[derive(Debug, Clone)]
pub struct ChatInfo {
    pub unseen_messages: usize,
    pub last_message: String,
    pub last_message_time: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct ChatContact {
    pub id: i32,
    mode: CardMode,
    pub contact: DbContact,
    profile_img_handle: image::Handle,
    chat_info: Option<ChatInfo>,
    messages_q: Vec<Message>,
    is_loading: bool,
}

impl ChatContact {
    pub fn new(id: i32, db_contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchChatInfo(db_contact.clone()));
        let size = ImageSize::Small;
        let profile_img_handle = db_contact.profile_image(size, conn);
        Self {
            id,
            mode: CardMode::Full,
            contact: db_contact.clone(),
            profile_img_handle,
            chat_info: None,
            messages_q: vec![],
            is_loading: true,
        }
    }
    pub fn view(&self, active_id: Option<i32>) -> Element<MessageWrapper> {
        let size = ImageSize::Small;
        let card_active = active_id.map(|id| id == self.id);
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
                    let last_message_cp: Element<_> = if let Some(chat_info) = &self.chat_info {
                        let content = &chat_info.last_message;
                        let left_pixels = size.width - NOTIFICATION_COUNT_WIDTH - 5.0; //spacing;
                        let pixel_p_char = 8.0; // 8px = 1 char
                        let taker = (left_pixels / pixel_p_char).floor() as usize;
                        let content = if taker > content.len() {
                            content.to_owned()
                        } else {
                            let truncated = content.graphemes(true).take(taker).collect::<String>();
                            format!("{}...", &truncated)
                        };
                        container(text(&content).size(18.0))
                            .width(Length::Fill)
                            .into()
                    } else {
                        text("").into()
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

    fn make_last_date<'a>(&'a self) -> Element<'a, MessageWrapper> {
        if let Some(chat_info) = &self.chat_info {
            let date = chat_info.last_message_time;
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
        } else {
            text("").into()
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match self.is_loading {
            true => match message {
                Message::GotChatInfo(chat_info) => {
                    self.handle_got_chat_info(chat_info, conn);
                }
                other => {
                    if self.is_loading {
                        self.messages_q.push(other);
                        return;
                    }
                }
            },
            false => match message {
                Message::GotChatInfo(chat_info) => {
                    self.handle_got_chat_info(chat_info, conn);
                }
                Message::ResetUnseenCount => {
                    if let Some(chat_info) = &mut self.chat_info {
                        chat_info.unseen_messages = 0;
                    }
                }
                Message::UpdatedMetadata(db_contact) | Message::ContactUpdated(db_contact) => {
                    self.profile_img_handle = db_contact.profile_image(ImageSize::Small, conn);
                    self.contact = db_contact;
                }
                Message::ContactPress(_) => (),
                Message::ShowOnlyProfileImage => {
                    self.mode = CardMode::Small;
                }
                Message::ShowFullCard => self.mode = CardMode::Full,
                Message::AddUnseenCount => {
                    if let Some(chat_info) = &mut self.chat_info {
                        chat_info.unseen_messages += 1;
                    }
                }
                Message::NewMessage(chat_msg) => {
                    if let Some(chat_info) = &mut self.chat_info {
                        chat_info.last_message = chat_msg.content;
                        chat_info.last_message_time = chat_msg.display_time;
                    }
                }
            },
        }
    }

    fn handle_got_chat_info(&mut self, chat_info: Option<ChatInfo>, conn: &mut BackEndConnection) {
        self.chat_info = chat_info;
        self.is_loading = false;
        // Move the messages_q vector out of self temporarily
        let mut messages = std::mem::replace(&mut self.messages_q, Vec::new());

        // Iterate over the messages and call self.update
        for msg in messages.drain(..) {
            self.update(msg, conn);
        }

        // Move the modified messages vector back into self
        self.messages_q = messages;
    }

    fn make_notifications<'a>(&self) -> Element<'a, MessageWrapper> {
        if let Some(chat_info) = &self.chat_info {
            match chat_info.unseen_messages {
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
        } else {
            text("").into()
        }
    }

    pub(crate) fn height(&self) -> f32 {
        CARD_HEIGHT
    }

    pub(crate) fn last_message_date(&self) -> Option<NaiveDateTime> {
        self.chat_info
            .as_ref()
            .map(|chat_info| chat_info.last_message_time)
    }
}

pub(crate) const CARD_HEIGHT: f32 = 80.0;
const NOTIFICATION_COUNT_WIDTH: f32 = 30.0;
