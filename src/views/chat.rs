use chrono::{Datelike, NaiveDateTime};
use iced::subscription::Subscription;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{alignment, Command, Length};

use crate::components::{contact_card, status_bar, StatusBar};
use crate::db::DbContact;
use crate::icon::{menu_bars_icon, send_icon};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::types::{chat_message, ChatMessage};
use crate::utils::contact_matches_search;
use crate::widget::{Column, Element};
use once_cell::sync::Lazy;

// static CONTACTS_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

#[derive(Debug, Clone)]
pub enum Message {
    StatusBarMessage(status_bar::Message),
    OnVerResize(u16),
    GoToChannelsPress,
    NavSettingsPress,
    ContactCardMessage(contact_card::Message),
    DMNMessageChange(String),
    DMSentPress,
    AddContactPress,
    ChatMessage(chat_message::Message),
    SearchContactInputChange(String),
    Scrolled(scrollable::RelativeOffset),
}

pub struct State {
    ver_divider_position: Option<u16>,
    contacts: Vec<contact_card::State>,
    active_contact: Option<DbContact>,
    dm_msg: String,
    messages: Vec<ChatMessage>,
    search_contact_input: String,
    show_only_profile: bool,
    current_scroll_offset: scrollable::RelativeOffset,
    status_bar: StatusBar,
}

impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            messages: vec![],
            ver_divider_position: Some(300),
            active_contact: None,
            dm_msg: "".into(),
            search_contact_input: "".into(),
            show_only_profile: false,
            current_scroll_offset: scrollable::RelativeOffset::END,
            status_bar: StatusBar::new(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.status_bar
            .subscription()
            .map(Message::StatusBarMessage)
    }

    pub fn view(&self) -> Element<Message> {
        // --- FIRST SPLIT ---
        let contact_list: Element<_> = if self.contacts.is_empty() {
            button("Add Contact")
                .on_press(Message::AddContactPress)
                .into()
        } else {
            scrollable(
                self.contacts
                    .iter()
                    .filter(|c| contact_matches_search(&c.contact, &self.search_contact_input))
                    .fold(column![].spacing(0), |col, contact| {
                        col.push(contact.view().map(Message::ContactCardMessage))
                    }),
            )
            .vertical_scroll(
                scrollable::Properties::new()
                    .width(6.0)
                    .margin(0.0)
                    .scroller_width(6.0),
            )
            // .id(CONTACTS_SCROLLABLE_ID.clone())
            .into()
        };
        let search_contact: Element<_> = match self.show_only_profile {
            true => text("").into(),
            false => text_input("Search", &self.search_contact_input)
                .on_input(Message::SearchContactInputChange)
                .style(style::TextInput::ChatSearch)
                .into(),
        };
        let menu_btn = button(menu_bars_icon())
            .on_press(Message::NavSettingsPress)
            .style(style::Button::Invisible);
        let search_container = container(
            row![menu_btn, search_contact]
                .spacing(5)
                .align_items(alignment::Alignment::Center),
        )
        .padding([10, 10])
        .width(Length::Fill)
        .height(NAVBAR_HEIGHT);
        let first = container(column![search_container, contact_list])
            .height(Length::Fill)
            .width(Length::Fixed(300.0))
            .style(style::Container::ContactList);
        // ---
        // --- SECOND SPLIT ---
        let chat_messages = create_chat_content(&self.messages);
        let message_input = text_input("Write a message...", &self.dm_msg)
            .on_submit(Message::DMSentPress)
            .on_input(Message::DMNMessageChange);
        let send_btn = button(send_icon().style(style::Text::Primary))
            .style(style::Button::Invisible)
            .on_press(Message::DMSentPress);
        let msg_input_row = container(row![message_input, send_btn].spacing(5))
            .style(style::Container::Default)
            .height(CHAT_INPUT_HEIGHT)
            .padding([10, 5]);

        let goto_channels_btn = button("Channels")
            .padding(5)
            .on_press(Message::GoToChannelsPress);

        let chat_navbar = container(
            row![Space::with_width(Length::Fill), goto_channels_btn]
                .spacing(5)
                .width(Length::Fill),
        )
        .padding(10)
        .height(NAVBAR_HEIGHT)
        .style(style::Container::Default);

        let second: Element<_> = if self.active_contact.is_some() {
            container(column![chat_navbar, chat_messages, msg_input_row])
                .width(Length::Fill)
                .into()
        } else {
            container(text("Select a chat to start messaging"))
                .center_x()
                .center_y()
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style::Container::ChatContainer)
                .into()
        };

        let main_content = container(row![first, second])
            .height(Length::Fill)
            .width(Length::Fill);
        let status_bar = self.status_bar.view().map(Message::StatusBarMessage);

        column![main_content, status_bar].into()

        // iced_aw::split::Split::new(
        //     first,
        //     second,
        //     self.ver_divider_position,
        //     iced_aw::split::Axis::Vertical,
        //     Message::OnVerResize,
        // )
        // .spacing(1.0)
        // .min_size_second(300)
        // .into()
    }
    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        let _command = self.status_bar.backend_event(event.clone(), conn);
        let command = match event {
            Event::GotContacts(db_contacts) => {
                self.contacts = db_contacts
                    .iter()
                    .map(|c| contact_card::State::from_db_contact(c))
                    .collect();
                Command::none()
            }
            Event::GotRelayResponses(responses) => {
                dbg!(responses);
                Command::none()
            }
            Event::GotChatMessages((db_contact, chat_msgs)) => {
                self.update_contact(db_contact.clone());

                if self.active_contact.as_ref() == Some(&db_contact) {
                    self.messages = chat_msgs;
                    self.messages
                        .sort_by(|a, b| a.created_at.cmp(&b.created_at));

                    self.current_scroll_offset = scrollable::RelativeOffset::END;
                    scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.current_scroll_offset)
                } else {
                    Command::none()
                }
            }
            Event::UpdateWithRelayResponse { db_message, .. } => {
                if let Some(msg) = &db_message {
                    if let Ok(msg_id) = msg.id() {
                        self.messages
                            .iter_mut()
                            .find(|m| m.msg_id == msg_id)
                            .map(|chat_msg| chat_msg.confirm_msg(msg));
                    }
                }
                Command::none()
            }
            Event::ReceivedDM((db_contact, msg)) => {
                self.update_contact(db_contact.clone());

                if self.active_contact.as_ref() == Some(&db_contact) {
                    if msg.status.is_offline() {
                        conn.send(net::Message::SendDMToRelays(msg.clone()));
                    }
                    // estou na conversa
                    self.messages.push(msg.clone());
                    self.messages
                        .sort_by(|a, b| a.created_at.cmp(&b.created_at));
                    self.current_scroll_offset = scrollable::RelativeOffset::END;

                    //COMMAND
                    scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.current_scroll_offset)
                } else {
                    // não estou na conversa
                    conn.send(net::Message::AddToUnseenCount(db_contact));
                    Command::none()
                }
            }
            Event::NewDMAndContact((db_contact, _)) => {
                self.contacts
                    .push(contact_card::State::from_db_contact(&db_contact));
                // não estou na conversa
                conn.send(net::Message::AddToUnseenCount(db_contact));
                Command::none()
            }
            Event::ContactUpdated(db_contact) => {
                self.update_contact(db_contact);
                Command::none()
            }
            _ => Command::none(),
        };

        command
    }
    fn update_contact(&mut self, db_contact: DbContact) {
        // change active to be an ID again...
        // println!("Update Contact: {}", &db_contact.pubkey().to_string());
        // dbg!(&db_contact);

        if self.active_contact.as_ref() == Some(&db_contact) {
            self.active_contact = Some(db_contact.clone());
        }

        if let Some(found_card) = self
            .contacts
            .iter_mut()
            .find(|c| c.contact.pubkey() == db_contact.pubkey())
        {
            found_card.update(contact_card::Message::ContactUpdated(db_contact.clone()));
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::StatusBarMessage(status_msg) => {
                let _command = self.status_bar.update(status_msg, conn);
            }
            // ---------
            Message::GoToChannelsPress => (),
            Message::Scrolled(offset) => {
                self.current_scroll_offset = offset;
            }
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ChatMessage(chat_msg) => match chat_msg {
                chat_message::Message::ChatRightClick(msg) => {
                    conn.send(net::Message::FetchRelayResponses(msg.event_id));
                    // dbg!(msg_clicked);
                }
            },
            Message::AddContactPress => (),
            Message::DMNMessageChange(text) => {
                self.dm_msg = text;
            }
            Message::DMSentPress => match (&self.active_contact, self.dm_msg.is_empty()) {
                (Some(contact), false) => {
                    conn.send(net::Message::BuildDM((
                        contact.to_owned(),
                        self.dm_msg.clone(),
                    )));
                    self.dm_msg = "".into();
                }
                _ => (),
            },

            Message::OnVerResize(position) => {
                if position > 200 && position < 400 {
                    self.ver_divider_position = Some(position);
                    self.show_only_profile = false;
                } else if position <= 200 && position > PIC_WIDTH {
                    self.ver_divider_position = Some(200);
                    self.show_only_profile = false;
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowFullCard);
                    }
                } else if position <= PIC_WIDTH {
                    self.ver_divider_position = Some(80);
                    self.show_only_profile = true;
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowOnlyProfileImage);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ContactCardMessage(card_msg) => {
                if let contact_card::Message::UpdateActiveContact(contact) = &card_msg {
                    if self.active_contact.as_ref() != Some(&contact) {
                        conn.send(net::Message::FetchMessages(contact.clone()));
                        self.messages = vec![];
                    }
                    self.dm_msg = "".into();
                    self.active_contact = Some(contact.clone());
                }

                for c in &mut self.contacts {
                    c.update(card_msg.clone());
                }
            }
        }
    }
}

fn chat_day_divider<Message: 'static>(date: NaiveDateTime) -> Element<'static, Message> {
    let text_container = container(text(date.format("%Y-%m-%d").to_string()))
        .style(style::Container::ChatDateDivider)
        .padding([5, 10]);
    container(text_container)
        .width(Length::Fill)
        .padding([20, 0])
        .center_x()
        .center_y()
        .into()
}

fn create_chat_content<'a>(messages: &[ChatMessage]) -> Element<'a, Message> {
    let chat_content: Element<_> = if messages.is_empty() {
        text("No messages").into()
    } else {
        let (chat_content, _): (Column<_>, Option<NaiveDateTime>) = messages.iter().fold(
            (column![], None),
            |(mut col, last_date), msg: &ChatMessage| {
                if let (Some(last_date), msg_date) = (last_date, msg.created_at) {
                    if last_date.day() != msg_date.day() {
                        col = col.push(chat_day_divider(msg_date.clone()));
                    }
                } else {
                    col = col.push(chat_day_divider(msg.created_at.clone()));
                }

                (
                    col.push(msg.view().map(Message::ChatMessage)),
                    Some(msg.created_at),
                )
            },
        );

        container(
            scrollable(chat_content)
                .height(Length::Fill)
                .vertical_scroll(
                    scrollable::Properties::new()
                        .width(6.0)
                        .margin(0.0)
                        .scroller_width(6.0),
                )
                .id(CHAT_SCROLLABLE_ID.clone())
                .on_scroll(Message::Scrolled),
        )
        .into()
    };

    container(chat_content)
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::ChatContainer)
        .into()
}

const PIC_WIDTH: u16 = 50;
const NAVBAR_HEIGHT: f32 = 50.0;
const CHAT_INPUT_HEIGHT: f32 = 50.0;
