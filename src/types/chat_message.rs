use chrono::NaiveDateTime;
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{alignment, Length};
use nostr_sdk::{secp256k1::XOnlyPublicKey, EventId};
use serde::{Deserialize, Serialize};

use crate::db::MessageStatus;
use crate::icon::{check_icon, double_check_icon, xmark_icon};
use crate::widget::Element;
use crate::{
    db::{DbContact, DbEvent, DbMessage},
    error::Error,
    style,
};

#[derive(Debug, Clone)]
pub enum Message {
    ChatRightClick(ChatMessage),
}

pub trait EventLike {
    fn created_at(&self) -> i64;
    fn pubkey(&self) -> XOnlyPublicKey;
}

impl EventLike for nostr_sdk::Event {
    fn created_at(&self) -> i64 {
        self.created_at.as_i64()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

impl EventLike for DbEvent {
    fn created_at(&self) -> i64 {
        self.created_at.timestamp_millis()
    }
    fn pubkey(&self) -> XOnlyPublicKey {
        self.pubkey.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub msg_id: i64,
    /// Message created at using unix timestamp
    pub created_at: NaiveDateTime,
    /// Decrypted message content
    pub content: String,
    pub is_from_user: bool,
    pub petname: Option<String>,
    pub event_id: i64,
    pub event_hash: EventId,
    pub status: MessageStatus,
}

impl ChatMessage {
    pub fn from_db_message(
        db_message: &DbMessage,
        is_from_user: bool,
        contact: &DbContact,
        content: &str,
    ) -> Result<Self, Error> {
        let msg_id = db_message.id()?;
        let event_id = db_message.event_id()?;
        let event_hash = db_message.event_hash()?;
        Ok(Self {
            content: content.to_owned(),
            created_at: db_message.created_at(),
            is_from_user,
            petname: contact.get_petname(),
            msg_id,
            event_id,
            event_hash,
            status: db_message.status(),
        })
    }

    pub fn confirm_msg(&mut self, db_message: &DbMessage) {
        self.status = db_message.status();
    }

    pub fn view(&self) -> Element<'static, Message> {
        let chat_alignment = match self.is_from_user {
            false => alignment::Horizontal::Left,
            true => alignment::Horizontal::Right,
        };

        // let card_padding = match self.is_from_user {
        //     false => [2, 100, 2, 20],
        //     true => [2, 20, 2, 100],
        // };

        let container_style = if self.is_from_user {
            style::Container::SentMessage
        } else {
            style::Container::ReceivedMessage
        };

        let time_str = self.created_at.time().format("%H:%M").to_string();
        let data_cp = column![
            // container(text("")).height(10.0),
            container(text(&time_str).style(style::Text::ChatMessageDate).size(16))
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

        let mouse_area =
            mouse_area(message_container).on_right_release(Message::ChatRightClick(self.clone()));

        container(mouse_area)
            .width(Length::Fill)
            .center_y()
            .align_x(chat_alignment)
            .padding([2, 20])
            .into()
    }
}

const CHAT_MESSAGE_MAX_WIDTH: f32 = 450.0;
