use crate::components::chat_contact::ChatContact;
use crate::components::{common_scrollable, Responsive};
use crate::consts::YMD_FORMAT;
use crate::icon::{file_icon_regular, send_icon};
use crate::style;
use crate::types::chat_message::{self, ChatMessage};
use crate::utils::from_naive_utc_to_local;
use crate::widget::{Button, Container, Element};
use chrono::{Datelike, NaiveDateTime};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Command, Length, Point, Size};

#[derive(Debug, Clone)]
pub enum Message {
    DMSentPress(String),
    DMNMessageChange(String),
    GotChatSize(Size, Size),
    Scrolled(scrollable::RelativeOffset),
    OpenContactProfile,
    ChatRightClick(ChatMessage, Point),
}

pub struct ChatListContainer {
    dm_msg_input: String,
}
impl ChatListContainer {
    pub fn new() -> Self {
        Self {
            dm_msg_input: "".into(),
        }
    }
    pub fn update_dm_msg(&mut self, text: String) {
        self.dm_msg_input = text;
    }
    pub fn view<'a>(
        &'a self,
        scrollable_id: &'a scrollable::Id,
        chat_input_id: &'a text_input::Id,
        messages: &'a [ChatMessage],
        active_chat: Option<&'a ChatContact>,
    ) -> Element<'a, Message> {
        if let Some(active_contact) = active_chat {
            let chat_messages = create_chat_content(scrollable_id, messages);
            let message_input = text_input("Write a message...", &self.dm_msg_input)
                .on_submit(Message::DMSentPress(self.dm_msg_input.clone()))
                .on_input(Message::DMNMessageChange)
                .id(chat_input_id.clone());
            let send_btn = button(send_icon().style(style::Text::Primary))
                .style(style::Button::Invisible)
                .on_press(Message::DMSentPress(self.dm_msg_input.clone()));
            let msg_input_row = container(row![message_input, send_btn].spacing(5))
                .style(style::Container::Default)
                .height(CHAT_INPUT_HEIGHT)
                .padding([10, 5]);
            // Todo: add/remove user button
            // if user is unkown
            let add_or_remove_user = text("");

            return container(column![
                chat_navbar(active_contact),
                add_or_remove_user,
                chat_messages,
                msg_input_row
            ])
            .width(Length::Fill)
            .into();
        }

        container(text("Select a chat to start messaging"))
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::Container::ChatContainer)
            .into()
    }
}

fn create_chat_content<'a>(
    scrollable_id: &'a scrollable::Id,
    messages: &'a [ChatMessage],
) -> Element<'a, Message> {
    let lazy = Responsive::new(move |_size| {
        if messages.is_empty() {
            return container(text("No messages"))
                .center_x()
                .center_y()
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style::Container::ChatContainer)
                .into();
        }

        let mut col = column![];
        let mut last_date: Option<NaiveDateTime> = None;

        for msg in messages {
            let msg_date = msg.display_time;

            if let Some(last) = last_date {
                if last.day() != msg_date.day() {
                    col = col.push(chat_day_divider(msg_date.clone()));
                }
            } else {
                col = col.push(chat_day_divider(msg_date.clone()));
            }

            col = col.push(msg.view().map(|m| match m {
                chat_message::Message::ChatRightClick(msg, point) => {
                    Message::ChatRightClick(msg, point)
                }
            }));

            last_date = Some(msg_date);
        }
        // col.into()
        let scrollable = common_scrollable(col)
            .height(Length::Fill)
            .id(scrollable_id.clone())
            .on_scroll(Message::Scrolled);

        scrollable.into()
    })
    .on_update(Message::GotChatSize);

    container(lazy)
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::ChatContainer)
        .into()
}

fn chat_day_divider<Message: 'static>(date: NaiveDateTime) -> Element<'static, Message> {
    let local_date = from_naive_utc_to_local(date);
    let text_container = container(text(local_date.format(YMD_FORMAT).to_string()))
        .style(style::Container::ChatDateDivider)
        .padding([5, 10]);
    container(text_container)
        .width(Length::Fill)
        .padding([20, 0])
        .center_x()
        .center_y()
        .into()
}

fn chat_navbar<'a>(active_contact: &'a ChatContact) -> Container<'a, Message> {
    container(
        row![header_details(active_contact), header_action_buttons()]
            .spacing(5)
            .width(Length::Fill),
    )
    .height(NAVBAR_HEIGHT)
    .style(style::Container::Default)
}

fn header_details<'a>(chat: &'a ChatContact) -> Button<'a, Message> {
    let local_message_date = chat
        .last_message_date()
        .map(from_naive_utc_to_local)
        .map(|date| format!("last seen {}", date.format(YMD_FORMAT)))
        .unwrap_or("".into());

    let user_name: Element<_> = if let Some(petname) = chat.contact.get_petname() {
        if !petname.is_empty() {
            row![
                text(petname).size(20),
                text(chat.contact.get_display_name().unwrap_or("".into()))
                    .size(14)
                    .style(style::Text::ChatMessageStatus)
            ]
            .align_items(Alignment::End)
            .spacing(5)
            .into()
        } else {
            text(chat.contact.select_name()).size(20).into()
        }
    } else {
        text(chat.contact.select_name()).size(20).into()
    };

    button(column![user_name, text(local_message_date).size(16)])
        .padding([5, 0, 0, 5])
        .style(style::Button::Invisible)
        .on_press(Message::OpenContactProfile)
        .height(Length::Fill)
        .width(Length::Fill)
}

fn header_action_buttons<'a>() -> Element<'a, Message> {
    row![button(file_icon_regular())
        .style(style::Button::Invisible)
        .on_press(Message::OpenContactProfile)]
    .padding(10)
    .align_items(Alignment::End)
    .into()
}

const NAVBAR_HEIGHT: f32 = 50.0;
const CHAT_INPUT_HEIGHT: f32 = 50.0;
