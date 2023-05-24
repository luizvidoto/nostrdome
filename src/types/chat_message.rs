use chrono::NaiveDateTime;
use iced::widget::{column, container, row, text, Space};
use iced::{alignment, Length};
use iced::{clipboard, Point};
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use serde::{Deserialize, Serialize};

use crate::components::MouseArea;
use crate::db::MessageStatus;
use crate::icon::{check_icon, double_check_icon, xmark_icon};
use crate::utils::from_naive_utc_to_local;
use crate::widget::Element;
use crate::{
    db::{DbContact, DbEvent, DbMessage},
    error::Error,
    style,
};

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ChatRightClick((ChatMessage, Point)),
}

pub trait EventLike {
    fn created_at(&self) -> i64;
    fn pubkey(&self) -> XOnlyPublicKey;
}

impl EventLike for nostr::Event {
    fn created_at(&self) -> i64 {
        self.created_at.as_i64()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

impl EventLike for DbEvent {
    fn created_at(&self) -> i64 {
        self.local_creation.timestamp_millis()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub msg_id: i64,
    pub display_time: NaiveDateTime,
    pub content: String,
    pub is_from_user: bool,
    pub select_name: String,
    pub event_id: i64,
    pub event_hash: EventId,
    pub status: MessageStatus,
}

impl ChatMessage {
    pub fn from_db_message(
        keys: &nostr::Keys,
        db_message: &DbMessage,
        contact: &DbContact,
    ) -> Result<Self, Error> {
        let content = db_message.decrypt_message(keys)?;
        Ok(Self::from_db_message_content(
            keys, db_message, contact, &content,
        )?)
    }

    pub fn from_db_message_content(
        keys: &nostr::Keys,
        db_message: &DbMessage,
        contact: &DbContact,
        content: &str,
    ) -> Result<Self, Error> {
        let is_from_user = db_message.im_author(&keys.public_key());
        let msg_id = db_message.id()?;
        let event_id = db_message.event_id()?;
        let event_hash = db_message.event_hash()?;
        Ok(Self {
            content: content.to_owned(),
            display_time: db_message.display_time(),
            is_from_user,
            select_name: contact.select_name(),
            msg_id,
            event_id,
            event_hash,
            status: db_message.status(),
        })
    }

    pub fn confirm_msg(&mut self, chat_msg: &ChatMessage) {
        self.display_time = chat_msg.display_time;
        self.status = chat_msg.status;
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let chat_alignment = match self.is_from_user {
            false => alignment::Horizontal::Left,
            true => alignment::Horizontal::Right,
        };

        let container_style = if self.is_from_user {
            style::Container::SentMessage
        } else {
            style::Container::ReceivedMessage
        };

        // TODO: to local timezone
        let local_time = from_naive_utc_to_local(self.display_time);
        let local_time = local_time.time().format("%H:%M").to_string();
        let data_cp = column![
            // container(text("")).height(10.0),
            container(
                text(&local_time)
                    .style(style::Text::ChatMessageDate)
                    .size(16)
            )
        ];

        let status = {
            let mut status = if self.is_from_user {
                match self.status {
                    MessageStatus::Offline => xmark_icon().size(14),
                    MessageStatus::Delivered => check_icon().size(14),
                    MessageStatus::Seen => double_check_icon().size(14),
                }
            } else {
                text("")
            };
            status = status.style(style::Text::ChatMessageDate);
            status
        };

        // text_input("", &self.content)
        //         .style(style::TextInput::Invisible)
        //         .on_input(|_| Message::None)
        //         .width(Length::Fill)
        //         .size(18)

        let msg_content = container(text(&self.content).size(18));
        let status_row = container(
            row![Space::new(Length::Shrink, Length::Shrink), data_cp, status]
                .spacing(5)
                .align_items(alignment::Alignment::Center),
        )
        .width(Length::Shrink);

        let message_container = container(column![msg_content, status_row].spacing(5))
            .width(Length::Shrink)
            .max_width(CHAT_MESSAGE_MAX_WIDTH)
            .padding([5, 10])
            .style(container_style);

        let mouse_area = MouseArea::new(message_container)
            .on_right_release(|p| Message::ChatRightClick((self.clone(), p)));

        container(mouse_area)
            .width(Length::Fill)
            .center_y()
            .align_x(chat_alignment)
            .padding([2, 20])
            .into()
    }
}

const CHAT_MESSAGE_MAX_WIDTH: f32 = 450.0;
