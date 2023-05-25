use chrono::{Datelike, NaiveDateTime};
use iced::clipboard;
use iced::subscription::Subscription;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::Size;
use iced::{alignment, Alignment, Command, Length};
use iced_native::widget::scrollable::RelativeOffset;
use nostr::secp256k1::XOnlyPublicKey;

use crate::components::chat_contact::{ChatContact, CARD_HEIGHT};
use crate::components::floating_element::{Anchor, FloatingElement, Offset};
use crate::components::{chat_contact, common_scrollable, status_bar, Responsive, StatusBar};
use crate::consts::YMD_FORMAT;
use crate::db::{DbContact, DbRelay, DbRelayResponse};
use crate::icon::{
    copy_icon, file_icon_regular, menu_bars_icon, reply_icon, satellite_icon, send_icon,
};
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::types::{chat_message, ChatMessage};
use crate::utils::{chat_matches_search, from_naive_utc_to_local};
use crate::widget::{Button, Container, Element};
use once_cell::sync::Lazy;

use super::modal::{basic_contact, relays_confirmation, ContactDetails, RelaysConfirmation};

static CONTACTS_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

// when profile modal is clicked, it sends the scrollable
// to the top but the state thinks that its on the bottom

pub enum ModalState {
    Off,
    BasicProfile(ContactDetails),
    RelaysConfirmation(RelaysConfirmation),
}
impl ModalState {
    pub fn basic_profile(contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        Self::BasicProfile(ContactDetails::viewer(contact, conn))
    }
    pub fn view<'a>(&'a self, underlay: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        match self {
            ModalState::Off => underlay.into(),
            ModalState::RelaysConfirmation(state) => state
                .view(underlay)
                .map(|m| Message::RelaysConfirmationMessage(Box::new(m))),
            ModalState::BasicProfile(state) => state
                .view(underlay)
                .map(|m| Message::BasicContactMessage(Box::new(m))),
        }
    }
    fn backend_event(&mut self, event: BackendEvent, conn: &mut BackEndConnection) {
        if let ModalState::BasicProfile(state) = self {
            state.backend_event(event, conn)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    CopyPressed,
    ReplyPressed,
    RelaysConfirmationPress,
    BasicContactMessage(Box<basic_contact::CMessage<Message>>),
    RelaysConfirmationMessage(Box<relays_confirmation::CMessage<Message>>),
    StatusBarMessage(status_bar::Message),
    OnVerResize(u16),
    GoToChannelsPress,
    NavSettingsPress,
    ContactCardMessage(chat_contact::MessageWrapper),
    DMNMessageChange(String),
    DMSentPress,
    AddContactPress,
    ChatMessage(chat_message::Message),
    SearchContactInputChange(String),
    Scrolled(scrollable::RelativeOffset),
    GotChatSize((Size, Size)),
    OpenContactProfile,
    CloseModal,
    CloseCtxMenu,
}

pub struct State {
    ver_divider_position: Option<u16>,
    chats: Vec<ChatContact>,
    active_idx: Option<i32>,
    dm_msg: String,
    messages: Vec<ChatMessage>,
    search_input: String,
    show_only_profile: bool,
    msgs_scroll_offset: scrollable::RelativeOffset,
    status_bar: StatusBar,
    modal_state: ModalState,
    context_menu_position: Offset,
    chat_window_size: Size,
    chat_total_size: Size,
    hide_context_menu: bool,
    chat_message_pressed: Option<ChatMessage>,
    last_relays_response: Option<RelaysResponse>,
    focus_pubkey: Option<XOnlyPublicKey>,
}

impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchContacts);
        Self {
            chats: Vec::new(),
            messages: vec![],
            ver_divider_position: Some(300),
            active_idx: None,
            dm_msg: "".into(),
            search_input: "".into(),
            show_only_profile: false,
            msgs_scroll_offset: scrollable::RelativeOffset::END,
            status_bar: StatusBar::new(),
            modal_state: ModalState::Off,
            context_menu_position: Offset { x: 0., y: 0. },
            chat_window_size: Size::ZERO,
            chat_total_size: Size::ZERO,
            hide_context_menu: true,
            chat_message_pressed: None,
            last_relays_response: None,
            focus_pubkey: None,
        }
    }
    pub(crate) fn chat_to(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> (Self, Command<Message>) {
        let mut state = Self::new(conn);
        state.focus_pubkey = Some(db_contact.pubkey().to_owned());
        (state, Command::none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.status_bar
            .subscription()
            .map(Message::StatusBarMessage)
    }

    fn active_chat(&self) -> Option<&ChatContact> {
        if let Some(idx) = self.active_idx {
            self.chats.iter().find(|c| c.id == idx)
        } else {
            None
        }
    }
    fn active_chat_mut(&mut self) -> Option<&mut ChatContact> {
        if let Some(idx) = self.active_idx {
            self.chats.iter_mut().find(|c| c.id == idx)
        } else {
            None
        }
    }
    fn active_matches(&self, db_contact: &DbContact) -> bool {
        if let Some(active_chat) = self.active_chat() {
            active_chat.contact.pubkey() == db_contact.pubkey()
        } else {
            false
        }
    }

    pub fn view(&self) -> Element<Message> {
        // --- FIRST SPLIT ---
        let contact_list: Element<_> = if self.chats.is_empty() {
            container(
                button("Add Contact")
                    .padding(10)
                    .on_press(Message::AddContactPress),
            )
            .center_x()
            .width(Length::Fill)
            .into()
        } else {
            let contact_list = self
                .chats
                .iter()
                .filter(|chat| chat_matches_search(&chat, &self.search_input))
                .fold(column![].spacing(0), |col, chat| {
                    col.push(chat.view(self.active_idx).map(Message::ContactCardMessage))
                });
            common_scrollable(contact_list)
                .id(CONTACTS_SCROLLABLE_ID.clone())
                .into()
        };
        let search_contact: Element<_> = match self.show_only_profile {
            true => text("").into(),
            false => text_input("Search", &self.search_input)
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
            .width(Length::Fill)
            .style(style::Container::ContactList);
        // ---
        // --- SECOND SPLIT ---
        let chat_messages = create_chat_content(&self.messages);
        let message_input = text_input("Write a message...", &self.dm_msg)
            .on_submit(Message::DMSentPress)
            .on_input(Message::DMNMessageChange)
            .id(CHAT_INPUT_ID.clone());
        let send_btn = button(send_icon().style(style::Text::Primary))
            .style(style::Button::Invisible)
            .on_press(Message::DMSentPress);
        let msg_input_row = container(row![message_input, send_btn].spacing(5))
            .style(style::Container::Default)
            .height(CHAT_INPUT_HEIGHT)
            .padding([10, 5]);

        let second: Element<_> = if let Some(active_contact) = self.active_chat() {
            // Todo: add/remove user button
            // if user is unkown
            let add_or_remove_user = text("");

            container(column![
                chat_navbar(active_contact),
                add_or_remove_user,
                chat_messages,
                msg_input_row
            ])
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

        let main_content = iced_aw::split::Split::new(
            first,
            second,
            self.ver_divider_position,
            iced_aw::split::Axis::Vertical,
            Message::OnVerResize,
        )
        .spacing(1.0)
        .min_size_second(300);
        let status_bar = self.status_bar.view().map(Message::StatusBarMessage);

        let underlay = column![main_content, status_bar];

        let float =
            FloatingElement::new(underlay, || make_context_menu(&self.last_relays_response))
                .on_esc(Message::CloseCtxMenu)
                .backdrop(Message::CloseCtxMenu)
                .anchor(Anchor::NorthWest)
                .offset(self.context_menu_position)
                .hide(self.hide_context_menu);

        self.modal_state.view(float)
    }

    fn handle_focus_contact(
        &mut self,
        conn: &mut BackEndConnection,
    ) -> Option<Vec<Command<Message>>> {
        if let Some(pubkey) = self.focus_pubkey.take() {
            let idx = self
                .chats
                .iter()
                .find(|chat| chat.contact.pubkey() == &pubkey)
                .map(|c| c.id);
            let position = self
                .chats
                .iter()
                .position(|c| c.contact.pubkey() == &pubkey);
            match (idx, position) {
                (Some(idx), Some(position)) => {
                    let mut commands = vec![self.set_active_contact(idx, conn)];
                    let list_height: f32 = self.chats.iter().map(|c| c.height()).sum();
                    let offset = calculate_scroll_offset(position, list_height, CARD_HEIGHT);
                    commands.push(scrollable::snap_to(CONTACTS_SCROLLABLE_ID.clone(), offset));
                    Some(commands)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn sort_contacts(&mut self) {
        self.chats
            .sort_by(|a, b| b.contact.select_name().cmp(&a.contact.select_name()));
        self.chats
            .sort_by(|a, b| b.last_message_date().cmp(&a.last_message_date()));
    }

    fn close_modal(&mut self) -> Command<Message> {
        self.modal_state = ModalState::Off;
        scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.msgs_scroll_offset)
    }

    fn set_active_contact(&mut self, idx: i32, conn: &mut BackEndConnection) -> Command<Message> {
        if let Some(chat) = self.chats.iter().find(|c| c.id == idx) {
            conn.send(net::ToBackend::FetchMessages(chat.contact.to_owned()));
            self.messages = vec![];
            self.dm_msg = "".into();
            self.active_idx = Some(idx);
            return text_input::focus(CHAT_INPUT_ID.clone());
        }
        Command::none()
    }

    fn find_and_update_contact<F>(
        &mut self,
        db_contact: &DbContact,
        message: chat_contact::Message,
        conn: &mut BackEndConnection,
        predicate: F,
    ) where
        F: Fn(&&mut ChatContact) -> bool,
    {
        if let Some(contact_card) = self.chats.iter_mut().find(predicate) {
            contact_card.update(message, conn);
        } else {
            let mut new_chat = ChatContact::new(self.chats.len() as i32, db_contact, conn);
            new_chat.update(message, conn);
            self.chats.push(new_chat);
        }
    }

    fn calculate_ctx_menu_pos(&mut self, point: iced_native::Point) {
        let total_h = self.chat_total_size.height;
        let window_h = self.chat_window_size.height;
        let offset_h = self.msgs_scroll_offset.y;
        let chat_window_offset_y = offset_h * window_h;
        let scroll_offset_y = offset_h * total_h;

        if total_h < window_h {
            self.context_menu_position = Offset {
                x: point.x,
                y: point.y,
            }
        } else {
            self.context_menu_position = Offset {
                x: point.x,
                y: point.y - scroll_offset_y + chat_window_offset_y,
            }
        }

        // check height for collision
        if window_h - (self.context_menu_position.y + ctx_menu_height()) < 0.0 {
            self.context_menu_position.y -= ctx_menu_height();
        }

        if let Some(div_pos) = self.ver_divider_position {
            //check width for collision
            if ((div_pos + 1) as f32) + self.chat_window_size.width
                - (self.context_menu_position.x + CONTEXT_MENU_WIDTH)
                < 0.0
            {
                self.context_menu_position.x -= CONTEXT_MENU_WIDTH;
            }
        }
    }

    fn move_contact_to_top(&mut self, db_contact: &DbContact) {
        // Get the position of the contact to be moved.
        if let Some(position) = find_contact_position(&self.chats, db_contact) {
            // If found, remove it from its current position.
            let contact_card = self.chats.remove(position);
            // Insert it at the top.
            self.chats.insert(0, contact_card);
        }
    }

    fn add_unseen_count(&mut self, db_contact: &DbContact, conn: &mut BackEndConnection) {
        self.find_and_update_contact(
            db_contact,
            chat_contact::Message::AddUnseenCount,
            conn,
            |c| c.contact.pubkey() == db_contact.pubkey(),
        );
    }

    pub fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        let mut commands = vec![];
        commands.push(
            self.status_bar
                .backend_event(event.clone(), conn)
                .map(Message::StatusBarMessage),
        );
        self.modal_state.backend_event(event.clone(), conn);

        match event {
            BackendEvent::ContactCreated(db_contact) => {
                let id = self.chats.len() as i32;
                self.chats
                    .push(chat_contact::ChatContact::new(id, &db_contact, conn));
            }
            BackendEvent::ContactUpdated(db_contact) => {
                self.find_and_update_contact(
                    &db_contact,
                    chat_contact::Message::ContactUpdated(db_contact.clone()),
                    conn,
                    |c| c.contact.pubkey() == db_contact.pubkey(),
                );
            }
            BackendEvent::ContactDeleted(db_contact) => {
                self.chats
                    .retain(|c| c.contact.pubkey() != db_contact.pubkey());
                self.active_idx = None;
            }
            BackendEvent::UpdatedMetadata(pubkey) => {
                conn.send(net::ToBackend::FetchSingleContact(pubkey));
            }
            BackendEvent::GotSingleContact((pubkey, db_contact)) => {
                if let Some(db_contact) = db_contact.as_ref() {
                    tracing::info!("Got single contact: {:?}", db_contact);
                    self.find_and_update_contact(
                        db_contact,
                        chat_contact::Message::ContactUpdated(db_contact.to_owned()),
                        conn,
                        |c| c.contact.pubkey() == &pubkey,
                    )
                }
            }
            BackendEvent::GotContacts(db_contacts) => {
                self.chats = db_contacts
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| chat_contact::ChatContact::new(idx as i32, c, conn))
                    .collect();

                if let Some(cmds) = self.handle_focus_contact(conn) {
                    cmds.into_iter().for_each(|c| commands.push(c));
                }
            }
            BackendEvent::GotRelayResponses {
                chat_message,
                responses,
                all_relays,
            } => {
                self.last_relays_response =
                    Some(RelaysResponse::new(chat_message, responses, all_relays));
            }
            BackendEvent::GotChatMessages((db_contact, chat_msgs)) => {
                if self.active_matches(&db_contact) {
                    if self.messages.is_empty() {
                        self.messages = chat_msgs;
                        self.msgs_scroll_offset = scrollable::RelativeOffset::END;
                        commands.push(scrollable::snap_to(
                            CHAT_SCROLLABLE_ID.clone(),
                            self.msgs_scroll_offset,
                        ));
                    } else {
                        // scrollable doesnt stay still when new messages are added at the top
                        self.messages.extend(chat_msgs);
                    }
                    self.messages
                        .sort_by(|a, b| a.display_time.cmp(&b.display_time));
                    self.active_chat_mut()
                        .map(|c| c.update(chat_contact::Message::ResetUnseenCount, conn));
                }
            }
            BackendEvent::ConfirmedDM((_db_contact, chat_msg)) => {
                self.messages
                    .iter_mut()
                    .find(|m| m.msg_id == chat_msg.msg_id)
                    .map(|m| m.confirm_msg(&chat_msg));
            }
            BackendEvent::UpdatedContactMetadata { db_contact, .. } => self
                .find_and_update_contact(
                    &db_contact,
                    chat_contact::Message::UpdatedMetadata(db_contact.clone()),
                    conn,
                    |c| c.contact.pubkey() == db_contact.pubkey(),
                ),
            BackendEvent::PendingDM((db_contact, msg)) => {
                self.messages.push(msg.clone());
                self.find_and_update_contact(
                    &db_contact,
                    chat_contact::Message::NewMessage(msg.clone()),
                    conn,
                    |c| c.contact.pubkey() == db_contact.pubkey(),
                );
                self.msgs_scroll_offset = scrollable::RelativeOffset::END;
                commands.push(scrollable::snap_to(
                    CHAT_SCROLLABLE_ID.clone(),
                    self.msgs_scroll_offset,
                ));
            }
            BackendEvent::ReceivedDM {
                chat_message: msg,
                db_contact,
                ..
            } => {
                if self.active_matches(&db_contact) {
                    // estou na conversa
                    self.messages.push(msg.clone());
                    self.msgs_scroll_offset = scrollable::RelativeOffset::END;
                    commands.push(scrollable::snap_to(
                        CHAT_SCROLLABLE_ID.clone(),
                        self.msgs_scroll_offset,
                    ));
                } else {
                    // não estou na conversa
                    self.move_contact_to_top(&db_contact);
                    self.add_unseen_count(&db_contact, conn);
                    commands.push(scrollable::snap_to(
                        CONTACTS_SCROLLABLE_ID.clone(),
                        scrollable::RelativeOffset::START,
                    ));
                }

                self.find_and_update_contact(
                    &db_contact,
                    chat_contact::Message::NewMessage(msg.clone()),
                    conn,
                    |c| c.contact.pubkey() == db_contact.pubkey(),
                );
            }

            BackendEvent::GotChatInfo((db_contact, chat_info)) => {
                self.find_and_update_contact(
                    &db_contact,
                    chat_contact::Message::GotChatInfo(chat_info),
                    conn,
                    |c| c.contact.pubkey() == db_contact.pubkey(),
                );
                let previous_position = self
                    .chats
                    .iter()
                    .position(|c| c.contact.pubkey() == db_contact.pubkey());

                self.sort_contacts();

                if let Some(prev_pos) = previous_position {
                    if let Some(current_position) = self
                        .chats
                        .iter()
                        .position(|c| c.contact.pubkey() == db_contact.pubkey())
                    {
                        // If the position has changed and the updated contact is the currently focused contact
                        if current_position != prev_pos
                            && self.focus_pubkey.as_ref() == Some(db_contact.pubkey())
                        {
                            // Recalculate the scroll offset
                            let list_height: f32 = self.chats.iter().map(|c| c.height()).sum();
                            let offset =
                                calculate_scroll_offset(current_position, list_height, CARD_HEIGHT);
                            commands
                                .push(scrollable::snap_to(CONTACTS_SCROLLABLE_ID.clone(), offset));
                        }
                    }
                }
            }

            _ => (),
        };

        Command::batch(commands)
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        let mut commands = vec![];
        match message {
            Message::CopyPressed => {
                if let Some(chat_msg) = &self.chat_message_pressed {
                    commands.push(clipboard::write(chat_msg.content.to_owned()));
                }
                self.hide_context_menu = true;
            }
            Message::ReplyPressed => {
                tracing::info!("Reply Pressed");
                self.hide_context_menu = true;
            }
            Message::CloseCtxMenu => {
                self.hide_context_menu = true;
            }
            Message::RelaysConfirmationPress => {
                // already have the relays responses
                self.hide_context_menu = true;
                if let Some(resp) = &self.last_relays_response {
                    self.modal_state = ModalState::RelaysConfirmation(RelaysConfirmation::new(
                        &resp.confirmed_relays,
                        &resp.all_relays,
                    ));
                }
            }
            Message::CloseModal => {
                commands.push(self.close_modal());
            }
            Message::OpenContactProfile => {
                if let Some(chat_contact) = self.active_chat() {
                    self.modal_state = ModalState::basic_profile(&chat_contact.contact, conn);
                }
            }
            Message::RelaysConfirmationMessage(modal_msg) => {
                if let ModalState::RelaysConfirmation(state) = &mut self.modal_state {
                    match *modal_msg {
                        relays_confirmation::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn);
                            if close_modal {
                                return self.close_modal();
                            }
                            commands
                                .push(cmd.map(|m| Message::RelaysConfirmationMessage(Box::new(m))));
                        }
                    }
                }
            }
            Message::BasicContactMessage(modal_msg) => {
                if let ModalState::BasicProfile(state) = &mut self.modal_state {
                    match *modal_msg {
                        basic_contact::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn);
                            if close_modal {
                                return self.close_modal();
                            }
                            commands.push(cmd.map(|m| Message::BasicContactMessage(Box::new(m))));
                        }
                    }
                }
            }

            Message::StatusBarMessage(status_msg) => {
                commands.push(
                    self.status_bar
                        .update(status_msg, conn)
                        .map(Message::StatusBarMessage),
                );
            }
            // ---------
            Message::GoToChannelsPress => (),
            Message::Scrolled(offset) => {
                self.msgs_scroll_offset = offset;

                // need to see if we need to fetch more messages
                if self.msgs_scroll_offset.y < 0.01 {
                    if let Some(active_chat) = self.active_chat() {
                        let contact = active_chat.contact.to_owned();
                        if let Some(first_message) = self.messages.first() {
                            tracing::info!("fetching more messages, first: {:?}", first_message);
                            conn.send(net::ToBackend::FetchMoreMessages((
                                contact,
                                first_message.to_owned(),
                            )));
                        }
                    }
                }
            }
            Message::GotChatSize((size, child_size)) => {
                self.chat_window_size = size;
                self.chat_total_size = child_size;
            }
            Message::SearchContactInputChange(text) => self.search_input = text,
            Message::ChatMessage(chat_msg) => match chat_msg {
                chat_message::Message::None => (),
                chat_message::Message::ChatRightClick((msg, point)) => {
                    conn.send(net::ToBackend::FetchRelayResponses(msg.clone()));
                    self.calculate_ctx_menu_pos(point);
                    self.hide_context_menu = false;
                    self.chat_message_pressed = Some(msg);
                }
            },
            Message::AddContactPress => (),
            Message::DMNMessageChange(text) => {
                self.dm_msg = text;
            }
            Message::DMSentPress => match (self.active_chat(), self.dm_msg.is_empty()) {
                (Some(chat_contact), false) => {
                    conn.send(net::ToBackend::SendDM((
                        chat_contact.contact.to_owned(),
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
                    for c in &mut self.chats {
                        c.update(chat_contact::Message::ShowFullCard, conn);
                    }
                } else if position <= PIC_WIDTH {
                    self.ver_divider_position = Some(80);
                    self.show_only_profile = true;
                    for c in &mut self.chats {
                        c.update(chat_contact::Message::ShowOnlyProfileImage, conn);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ContactCardMessage(wrapped) => {
                if let chat_contact::Message::ContactPress(idx) = &wrapped.message.clone() {
                    commands.push(self.set_active_contact(*idx, conn));
                }
            }
        }

        Command::batch(commands)
    }
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

fn create_chat_content<'a>(messages: &'a [ChatMessage]) -> Element<'a, Message> {
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

            col = col.push(msg.view().map(Message::ChatMessage));
            last_date = Some(msg_date);
        }
        // col.into()
        let scrollable = common_scrollable(col)
            .height(Length::Fill)
            .id(CHAT_SCROLLABLE_ID.clone())
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
    .align_items(alignment::Alignment::End)
    .into()
}

fn ctx_menu_height() -> f32 {
    let n = 3.0;
    let padding = 0.0;
    let ctx_elements_h = (CTX_BUTTON_HEIGHT + padding * 2.0) * n;

    let times = n - 1.0;
    let spacing = 5.0;
    let ctx_spacing_h = times * spacing;

    let menu_padding = 10.0;

    ctx_elements_h + ctx_spacing_h + menu_padding
}

fn make_context_menu<'a>(response: &Option<RelaysResponse>) -> Element<'a, Message> {
    let copy_btn = button(
        row![
            text("Copy").size(18),
            Space::with_width(Length::Fill),
            copy_icon().size(16)
        ]
        .align_items(Alignment::Center),
    )
    .width(Length::Fill)
    .height(CTX_BUTTON_HEIGHT)
    .on_press(Message::CopyPressed)
    .style(style::Button::ContextMenuButton);
    let reply_btn = button(
        row![
            text("Reply").size(18),
            Space::with_width(Length::Fill),
            reply_icon().size(16)
        ]
        .align_items(Alignment::Center),
    )
    .height(CTX_BUTTON_HEIGHT)
    .width(Length::Fill)
    // .on_press(Message::ReplyPressed)
    .style(style::Button::ContextMenuButton);
    let relays_btn: Element<_> = if let Some(response) = response {
        let resp_txt = format!(
            "{}/{}",
            &response.confirmed_relays.len(),
            &response.all_relays.len()
        );
        button(
            row![
                text("Relays").size(18),
                Space::with_width(Length::Fill),
                text(&resp_txt).size(18),
                satellite_icon().size(16)
            ]
            .align_items(Alignment::Center)
            .spacing(2),
        )
        .width(Length::Fill)
        .height(CTX_BUTTON_HEIGHT)
        .on_press(Message::RelaysConfirmationPress)
        .style(style::Button::ContextMenuButton)
        .into()
    } else {
        container(text("No confirmation"))
            .width(Length::Fill)
            .height(CTX_BUTTON_HEIGHT)
            .into()
    };

    let buttons = column![copy_btn, reply_btn, relays_btn].spacing(5);

    container(buttons)
        .height(ctx_menu_height())
        .width(CONTEXT_MENU_WIDTH)
        .style(style::Container::ContextMenu)
        .padding(5)
        .into()
}

// Function to find the position of a contact in the contacts list
fn find_contact_position(contacts: &[ChatContact], db_contact: &DbContact) -> Option<usize> {
    contacts
        .iter()
        .position(|card| card.contact.pubkey() == db_contact.pubkey())
}

// Function to calculate the scroll offset for a given position
fn calculate_scroll_offset(position: usize, total_height: f32, card_height: f32) -> RelativeOffset {
    let card_list_height = position as f32 * card_height + card_height;
    let y = card_list_height / total_height;
    RelativeOffset { x: 0.0, y }
}

#[derive(Debug, Clone)]
pub struct RelaysResponse {
    pub confirmed_relays: Vec<DbRelayResponse>,
    pub all_relays: Vec<DbRelay>,
    pub chat_message: ChatMessage,
}
impl RelaysResponse {
    fn new(
        chat_message: ChatMessage,
        confirmed_relays: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    ) -> RelaysResponse {
        Self {
            chat_message,
            confirmed_relays,
            all_relays,
        }
    }
}

const PIC_WIDTH: u16 = 50;
const NAVBAR_HEIGHT: f32 = 50.0;
const CHAT_INPUT_HEIGHT: f32 = 50.0;
const CONTEXT_MENU_WIDTH: f32 = 130.0;
const CTX_BUTTON_HEIGHT: f32 = 30.0;
