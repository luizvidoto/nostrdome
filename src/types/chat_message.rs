use chrono::NaiveDateTime;
use iced::widget::{column, container, row, text, Space};
use iced::Point;
use iced::{alignment, Length};
use nostr::EventId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::components::MouseArea;
use crate::db::MessageStatus;
use crate::icon::{check_icon, double_check_icon, xmark_icon};
use crate::utils::from_naive_utc_to_local;
use crate::widget::Element;
use crate::{
    db::{DbContact, DbMessage},
    style,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    FromDbMessageError(#[from] crate::db::message::Error),
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ChatRightClick((ChatMessage, Point)),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub msg_id: i64,
    pub display_time: NaiveDateTime,
    pub content: String,
    pub is_from_user: bool,
    pub select_name: String,
    pub event_id: Option<i64>,
    pub event_hash: EventId,
    pub status: MessageStatus,
}

impl ChatMessage {
    pub fn new(db_message: &DbMessage, contact: &DbContact, content: &str) -> Result<Self, Error> {
        Ok(Self {
            content: content.to_owned(),
            display_time: db_message.display_time(),
            is_from_user: db_message.is_users,
            select_name: contact.select_name(),
            msg_id: db_message.id,
            event_hash: db_message.event_hash,
            event_id: db_message.confirmation_info.as_ref().map(|c| c.event_id),
            status: db_message.status,
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
        let data_cp = column![container(
            text(&local_time)
                .style(style::Text::ChatMessageDate)
                .size(16)
        )];

        let status = {
            let mut status = if self.is_from_user {
                match self.status {
                    MessageStatus::Pending => xmark_icon().size(14),
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
