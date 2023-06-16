use chrono::NaiveDateTime;
use iced::widget::{button, column, container, row, text};
use iced::Point;
use iced::{alignment, Length};
use nostr::secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::components::MouseArea;
use crate::db::{DbChannelMessage, MessageStatus};
use crate::icon::{check_icon, double_check_icon, xmark_icon};
use crate::utils::{from_naive_utc_to_local, hide_string};
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
    ChatRightClick(ChatMessage, Point),
    UserNameClick(XOnlyPublicKey),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub msg_id: i64,
    pub content: String,
    pub author: XOnlyPublicKey,
    pub is_users: bool,
    pub display_name: String,
    pub display_time: NaiveDateTime,
    pub event_id: Option<i64>,
    pub status: MessageStatus,
}

impl ChatMessage {
    pub fn new(
        db_message: &DbMessage,
        author: &XOnlyPublicKey,
        contact: &DbContact,
        content: &str,
    ) -> Self {
        Self {
            msg_id: db_message.event_id,
            content: content.to_owned(),
            author: author.to_owned(),
            display_time: db_message.created_at.to_owned(),
            is_users: db_message.is_users,
            display_name: contact.select_name(),
            event_id: Some(db_message.event_id),
            status: db_message.status,
        }
    }

    pub fn confirm_msg(&mut self, chat_msg: &ChatMessage) {
        self.display_time = chat_msg.display_time;
        self.status = MessageStatus::Seen;
    }

    pub fn show_name(&self, previous_msg: Option<Self>) -> bool {
        if self.is_users {
            return false;
        }
        if let Some(previous_msg) = previous_msg {
            // only shows name if user is different
            previous_msg.display_name != self.display_name
        } else {
            // if no previous message, show name
            true
        }
    }

    pub fn view<'a>(&'a self, show_name: bool) -> Element<'a, Message> {
        let chat_alignment = match self.is_users {
            false => alignment::Horizontal::Left,
            true => alignment::Horizontal::Right,
        };

        let container_style = if self.is_users {
            style::Container::SentMessage
        } else {
            style::Container::ReceivedMessage
        };

        let local_time = from_naive_utc_to_local(self.display_time);
        let local_time = local_time.time().format("%H:%M").to_string();
        let local_time = text(&local_time).style(style::Text::Alpha(0.5)).size(16);

        let status = if self.is_users {
            match self.status {
                MessageStatus::Pending => xmark_icon().size(14),
                MessageStatus::Delivered => check_icon().size(14),
                MessageStatus::Seen => double_check_icon().size(14),
            }
        } else {
            text("")
        };
        let status = status.style(style::Text::Alpha(0.5));

        // only shows name if is in channel view and
        // previous chat message is a different user
        let name: Element<_> = if show_name {
            button(text(&self.display_name))
                .on_press(Message::UserNameClick(self.author.clone()))
                .style(style::Button::Invisible)
                .into()
        } else {
            text("").into()
        };

        let content = text(&self.content).size(18);
        let status_row = row![local_time, status].spacing(5);

        let message_container = column![name, content, status_row]
            // this works but all the items are aligned to the right
            // and I cant realign them to the left after this
            // .align_items(alignment::Alignment::End)
            .spacing(5);
        let message_container = container(message_container)
            .max_width(CHAT_MESSAGE_MAX_WIDTH)
            .padding([5, 10])
            .style(container_style);

        let mouse_area = MouseArea::new(message_container)
            .on_right_release(|p| Message::ChatRightClick(self.clone(), p));

        container(mouse_area)
            .width(Length::Fill)
            .center_y()
            .align_x(chat_alignment)
            .padding([2, 20])
            .into()
    }
}

impl From<DbChannelMessage> for ChatMessage {
    fn from(ch_msg: DbChannelMessage) -> Self {
        let display_name = hide_string(&ch_msg.display_name(), 6);
        Self {
            msg_id: ch_msg.event_id,
            author: ch_msg.author,
            display_time: ch_msg.created_at,
            content: ch_msg.content,
            is_users: ch_msg.is_users,
            display_name,
            event_id: Some(ch_msg.event_id),
            status: MessageStatus::Delivered,
        }
    }
}

const CHAT_MESSAGE_MAX_WIDTH: f32 = 450.0;
