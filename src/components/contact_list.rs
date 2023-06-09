use crate::components::chat_contact::{self, ChatContact};
use crate::components::common_scrollable;
use crate::style;
use crate::utils::chat_matches_search;
use crate::widget::Element;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{alignment, Length};

#[derive(Debug, Clone)]
pub enum Message {
    AddContactPress,
    SearchContactInputChange(String),
    ContactPress(i32),
}
pub struct ContactList {
    search_input: String,
}
impl ContactList {
    pub fn new() -> Self {
        Self {
            search_input: "".into(),
        }
    }
    pub fn search_input_change(&mut self, text: String) {
        self.search_input = text;
    }
    pub fn view<'a>(
        &'a self,
        scrollable_id: &'a scrollable::Id,
        chats: &'a [ChatContact],
        show_only_profile: bool,
        active_idx: Option<i32>,
    ) -> Element<'a, Message> {
        // --- FIRST SPLIT ---
        let contact_list: Element<_> = if chats.is_empty() {
            container(
                button("Add Contact")
                    .padding(10)
                    .on_press(Message::AddContactPress),
            )
            .center_x()
            .width(Length::Fill)
            .into()
        } else {
            let contact_list = chats
                .iter()
                .filter(|chat| chat_matches_search(&chat, &self.search_input))
                .fold(column![].spacing(0), |col, chat| {
                    col.push(chat.view(active_idx).map(|m| match m.message {
                        chat_contact::Message::ContactPress(idx) => Message::ContactPress(idx),
                    }))
                });
            common_scrollable(contact_list)
                .id(scrollable_id.clone())
                .into()
        };
        let search_contact: Element<_> = match show_only_profile {
            true => text("").into(),
            false => text_input("Search", &self.search_input)
                .on_input(Message::SearchContactInputChange)
                .style(style::TextInput::ChatSearch)
                .into(),
        };
        let search_container = container(
            row![search_contact]
                .spacing(5)
                .align_items(alignment::Alignment::Center),
        )
        .padding([10, 10])
        .width(Length::Fill)
        .height(NAVBAR_HEIGHT);

        container(column![search_container, contact_list])
            .height(Length::Fill)
            .width(Length::Fill)
            .style(style::Container::ContactList)
            .into()
    }
}

const NAVBAR_HEIGHT: f32 = 50.0;
