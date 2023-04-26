use chrono::NaiveDateTime;
use iced::widget::{column, container, mouse_area, row, text};
use iced::{alignment, Length};
use nostr_sdk::{secp256k1::XOnlyPublicKey, EventId};
use serde::{Deserialize, Serialize};

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
    /// Pub key of the author of the message
    pub from_pubkey: XOnlyPublicKey,
    pub is_from_user: bool,
    pub petname: Option<String>,
    pub event_id: i64,
    pub event_hash: EventId,
}

impl ChatMessage {
    pub fn from_db_message(
        db_message: &DbMessage,
        is_from_user: bool,
        contact: &DbContact,
        content: &str,
    ) -> Result<Self, Error> {
        let msg_id = db_message.msg_id()?;
        let event_id = db_message.event_id()?;
        let event_hash = db_message.event_hash()?;
        Ok(Self {
            content: content.to_owned(),
            created_at: db_message.created_at(),
            from_pubkey: db_message.from_pubkey(),
            is_from_user,
            petname: contact.get_petname(),
            msg_id,
            event_id,
            event_hash,
        })
    }

    pub fn view(&self) -> Element<'static, Message> {
        let chat_alignment = match self.is_from_user {
            false => alignment::Horizontal::Left,
            true => alignment::Horizontal::Right,
        };

        let container_style = if self.is_from_user {
            style::Container::SentMessage
        } else {
            style::Container::ReceivedMessage
        };

        let time_str = self.created_at.time().format("%H:%M").to_string();
        let data_cp = column![
            // container(text("")).height(10.0),
            container(text(&time_str).style(style::Text::Placeholder).size(14))
        ];

        let msg_content = text(&self.content).size(18);

        let message_container = container(row![msg_content, data_cp].spacing(5))
            .padding([5, 10])
            .style(container_style);

        let container = container(message_container)
            .width(Length::Fill)
            .center_y()
            .align_x(chat_alignment)
            .padding([2, 20]);

        mouse_area(container)
            .on_right_release(Message::ChatRightClick(self.clone()))
            .into()
    }
}
