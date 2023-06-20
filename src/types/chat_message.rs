use chrono::NaiveDateTime;
use iced::widget::{button, column, container, row, text};
use iced::Point;
use iced::{alignment, Length};
use nostr::secp256k1::XOnlyPublicKey;
use nostr::EventId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::components::MouseArea;
use crate::db::{DbChannelMessage, MessageStatus};
use crate::icon::{check_icon, double_check_icon, xmark_icon};
use crate::utils::{from_naive_utc_to_local, hide_string};
use crate::widget::{Element, Text};
use crate::{
    db::{DbContact, DbMessage},
    style,
};

use super::PendingEvent;

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
pub enum UserMessage {
    Pending {
        event_hash: EventId,
        content: String,
        display_time: Option<NaiveDateTime>,
    },
    Confirmed {
        content: String,
        display_time: NaiveDateTime,
        event_id: i64,
        status: MessageStatus,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessage {
    UserMessage(UserMessage),
    ContactMessage {
        content: String,
        author: XOnlyPublicKey,
        display_name: String,
        display_time: NaiveDateTime,
        event_id: i64,
        status: MessageStatus,
    },
}

impl ChatMessage {
    pub fn is_pending(&self) -> bool {
        if let Self::UserMessage(UserMessage::Pending { .. }) = self {
            return true;
        }
        false
    }
    pub fn match_pending_hash(&self, event_hash: &EventId) -> bool {
        if let Self::UserMessage(UserMessage::Pending {
            event_hash: pending_hash,
            ..
        }) = self
        {
            return pending_hash == event_hash;
        }
        false
    }
    pub fn event_id(&self) -> Option<i64> {
        match self {
            Self::UserMessage(user) => match user {
                UserMessage::Pending { .. } => None,
                UserMessage::Confirmed { event_id, .. } => Some(*event_id),
            },
            Self::ContactMessage { event_id, .. } => Some(*event_id),
        }
    }
    pub fn pending(pending: PendingEvent, content: &str) -> Self {
        let user_msg = UserMessage::Pending {
            event_hash: pending.event_hash().to_owned(),
            content: content.to_owned(),
            display_time: pending.display_time().ok(),
        };
        Self::UserMessage(user_msg)
    }

    pub fn confirmed_users(db_message: &DbMessage, content: &str) -> Self {
        let user_msg = UserMessage::Confirmed {
            content: content.to_owned(),
            display_time: db_message.created_at.to_owned(),
            event_id: db_message.event_id,
            status: db_message.status,
        };
        Self::UserMessage(user_msg)
    }
    pub fn confirmed_contacts(db_message: &DbMessage, contact: &DbContact, content: &str) -> Self {
        Self::ContactMessage {
            content: content.to_owned(),
            author: contact.pubkey().to_owned(),
            display_time: db_message.created_at.to_owned(),
            display_name: contact.select_name(),
            event_id: db_message.event_id,
            status: db_message.status,
        }
    }

    pub fn show_name(&self, previous_msg: Option<&Self>) -> bool {
        match self {
            Self::UserMessage { .. } => false,
            Self::ContactMessage { display_name, .. } => match previous_msg {
                Some(Self::UserMessage { .. }) => true,
                Some(Self::ContactMessage {
                    display_name: prev_display_name,
                    ..
                }) => prev_display_name != display_name,
                None => true,
            },
        }
    }

    fn alignment(&self) -> alignment::Horizontal {
        match self {
            ChatMessage::ContactMessage { .. } => alignment::Horizontal::Left,
            ChatMessage::UserMessage(_) => alignment::Horizontal::Right,
        }
    }

    fn style(&self) -> style::Container {
        match self {
            ChatMessage::ContactMessage { .. } => style::Container::ReceivedMessage,
            ChatMessage::UserMessage(_) => style::Container::SentMessage,
        }
    }

    fn status<'a, M: 'a>(&'a self) -> Element<'a, M> {
        let style = match self {
            ChatMessage::ContactMessage { .. } => check_icon().size(14),
            ChatMessage::UserMessage(user) => match user {
                UserMessage::Pending { .. } => xmark_icon().size(14),
                UserMessage::Confirmed { status, .. } => match status {
                    MessageStatus::Pending => xmark_icon().size(14),
                    MessageStatus::Delivered => check_icon().size(14),
                    MessageStatus::Seen => double_check_icon().size(14),
                },
            },
        };
        style.style(style::Text::Alpha(0.5)).into()
    }

    pub fn display_time(&self) -> Option<&NaiveDateTime> {
        match self {
            ChatMessage::UserMessage(user) => match user {
                UserMessage::Pending { display_time, .. } => display_time.as_ref(),
                UserMessage::Confirmed { display_time, .. } => Some(display_time),
            },
            ChatMessage::ContactMessage { display_time, .. } => Some(display_time),
        }
    }

    fn local_time(&self) -> Text<'_> {
        make_local_time(self.display_time())
    }

    fn name(&self, show_name: bool) -> Element<'_, Message> {
        if !show_name {
            return text("").into();
        }

        match self {
            ChatMessage::UserMessage(_) => text("").into(),
            ChatMessage::ContactMessage {
                display_name,
                author,
                ..
            } => {
                // only shows name if is in channel view and
                // previous chat message is a different user
                button(text(display_name))
                    .on_press(Message::UserNameClick(*author))
                    .style(style::Button::Invisible)
                    .into()
            }
        }
    }
    pub fn content(&self) -> &str {
        match self {
            ChatMessage::UserMessage(user) => match user {
                UserMessage::Pending { content, .. } => content,
                UserMessage::Confirmed { content, .. } => content,
            },
            ChatMessage::ContactMessage { content, .. } => content,
        }
    }

    pub fn view(&self, show_name: bool) -> Element<'_, Message> {
        make_chat_view(
            self.alignment(),
            self.style(),
            self.name(show_name),
            self.status(),
            self.local_time(),
            self.content(),
            |p| Message::ChatRightClick(self.clone(), p),
        )
    }

    pub(crate) fn update_display_name(&mut self, pubkey: &XOnlyPublicKey, name: String) {
        match self {
            ChatMessage::UserMessage(_) => (),
            ChatMessage::ContactMessage {
                author,
                display_name,
                ..
            } => {
                if author == pubkey {
                    *display_name = name;
                }
            }
        }
    }
}

fn make_local_time<'a>(display_time: Option<&NaiveDateTime>) -> Text<'a> {
    if let Some(display_time) = display_time {
        let local_time = from_naive_utc_to_local(*display_time);
        let local_time = local_time.time().format("%H:%M").to_string();
        text(&local_time).style(style::Text::Alpha(0.5)).size(16)
    } else {
        text("")
    }
}

fn make_chat_view<'a, F>(
    alignment: alignment::Horizontal,
    container_style: style::Container,
    name: impl Into<Element<'a, Message>>,
    status: impl Into<Element<'a, Message>>,
    local_time: impl Into<Element<'a, Message>>,
    content: &'a str,
    on_right_press: F,
) -> Element<'a, Message>
where
    F: 'a + Fn(Point) -> Message,
{
    let content = text(content).size(18);
    let status_row = row![local_time.into(), status.into()].spacing(5);
    let message_container = column![name.into(), content, status_row]
        // this works but all the items are aligned to the right
        // and I cant realign them to the left after this
        // .align_items(alignment::Alignment::End)
        .spacing(5);

    let message_container = container(message_container)
        .max_width(CHAT_MESSAGE_MAX_WIDTH)
        .padding([5, 10])
        .style(container_style);

    let mouse_area = MouseArea::new(message_container).on_right_release(on_right_press);

    container(mouse_area)
        .width(Length::Fill)
        .center_y()
        .align_x(alignment)
        .padding([2, 20])
        .into()
}

impl From<DbChannelMessage> for ChatMessage {
    fn from(ch_msg: DbChannelMessage) -> Self {
        if ch_msg.is_users {
            Self::UserMessage(UserMessage::Confirmed {
                content: ch_msg.content,
                display_time: ch_msg.created_at,
                event_id: ch_msg.event_id,
                status: MessageStatus::Delivered,
            })
        } else {
            let display_name = hide_string(&ch_msg.display_name(), 6);
            Self::ContactMessage {
                author: ch_msg.author,
                display_time: ch_msg.created_at,
                content: ch_msg.content,
                display_name,
                event_id: ch_msg.event_id,
                status: MessageStatus::Delivered,
            }
        }
    }
}

const CHAT_MESSAGE_MAX_WIDTH: f32 = 450.0;
