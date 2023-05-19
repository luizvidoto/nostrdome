use std::path::PathBuf;

use chrono::{Datelike, NaiveDateTime};
use iced::clipboard;
use iced::subscription::Subscription;
use iced::widget::{
    button, column, container, image, row, scrollable, text, text_input, Image, Space,
};
use iced::Size;
use iced::{alignment, Alignment, Command, Length};
use iced_aw::{Card, Modal};
use iced_native::widget::scrollable::RelativeOffset;

use crate::components::contact_card::ContactCard;
use crate::components::floating_element::{Anchor, FloatingElement, Offset};
use crate::components::text::title;
use crate::components::{common_scrollable, contact_card, status_bar, Responsive, StatusBar};
use crate::consts::{DEFAULT_PROFILE_IMAGE_SMALL, YMD_FORMAT};
use crate::db::{DbContact, DbRelay, DbRelayResponse};
use crate::icon::{
    copy_icon, file_icon_regular, menu_bars_icon, reply_icon, satellite_icon, send_icon,
};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::types::{chat_message, ChatMessage};
use crate::utils::{contact_matches_search_selected_name, from_naive_utc_to_local};
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
    Profile {
        name: String,
        profile: nostr_sdk::Metadata,
        profile_image_handle: image::Handle,
    },
    RelaysConfirmation(RelaysConfirmation),
}
impl ModalState {
    pub fn basic_profile(contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        Self::BasicProfile(ContactDetails::viewer(contact, conn))
    }
    pub fn view<'a>(&'a self, underlay: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        let view: Element<_> = match self {
            ModalState::Off => underlay.into(),
            ModalState::RelaysConfirmation(state) => state
                .view(underlay)
                .map(|m| Message::RelaysConfirmationMessage(Box::new(m))),
            ModalState::BasicProfile(state) => state
                .view(underlay)
                .map(|m| Message::BasicContactMessage(Box::new(m))),
            ModalState::Profile {
                name,
                profile,
                profile_image_handle,
            } => {
                let _profile_image = Image::new(profile_image_handle.to_owned());
                let modal = Modal::new(true, underlay, move || {
                    let title = title(name);
                    let mut content = column![].spacing(5);
                    if let Some(name) = &profile.name {
                        content = content.push(column![text("name"), text(name)].spacing(5));
                    }
                    if let Some(display_name) = &profile.display_name {
                        content = content
                            .push(column![text("display_name"), text(display_name)].spacing(5));
                    }
                    if let Some(picture_url) = &profile.picture {
                        content = content
                            .push(column![text("picture_url"), text(picture_url)].spacing(5));
                    }
                    if let Some(about) = &profile.about {
                        content = content.push(column![text("about"), text(about)].spacing(5));
                    }
                    if let Some(website) = &profile.website {
                        content = content.push(column![text("website"), text(website)].spacing(5));
                    }
                    if let Some(banner_url) = &profile.banner {
                        content =
                            content.push(column![text("banner_url"), text(banner_url)].spacing(5));
                    }
                    if let Some(nip05) = &profile.nip05 {
                        content = content.push(column![text("nip05"), text(nip05)].spacing(5));
                    }
                    if let Some(lud06) = &profile.lud06 {
                        content = content.push(column![text("lud06"), text(lud06)].spacing(5));
                    }
                    if let Some(lud16) = &profile.lud16 {
                        content = content.push(column![text("lud16"), text(lud16)].spacing(5));
                    }

                    let modal_body = content;
                    Card::new(title, modal_body)
                        .max_width(MODAL_WIDTH)
                        .on_close(Message::CloseModal)
                        .into()
                })
                .backdrop(Message::CloseModal)
                .on_esc(Message::CloseModal);
                modal.into()
            }
        };

        view
    }

    fn contact_profile(
        select_name: String,
        profile_meta: nostr_sdk::Metadata,
        profile_image_path: Option<&PathBuf>,
    ) -> Self {
        let profile_image_handle = if let Some(path) = profile_image_path {
            image::Handle::from_path(path)
        } else {
            image::Handle::from_memory(DEFAULT_PROFILE_IMAGE_SMALL)
        };
        Self::Profile {
            name: select_name,
            profile: profile_meta,
            profile_image_handle,
        }
    }

    fn backend_event(&mut self, event: Event, conn: &mut BackEndConnection) {
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
    ContactCardMessage(contact_card::MessageWrapper),
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
    contacts: Vec<contact_card::ContactCard>,
    active_contact: Option<DbContact>,
    dm_msg: String,
    messages: Vec<ChatMessage>,
    search_contact_input: String,
    show_only_profile: bool,
    current_scroll_offset: scrollable::RelativeOffset,
    status_bar: StatusBar,
    modal_state: ModalState,
    context_menu_position: Offset,
    chat_window_size: Size,
    chat_total_size: Size,
    hide_context_menu: bool,
    chat_message_pressed: Option<ChatMessage>,
    last_relays_response: Option<RelaysResponse>,
    focus_contact: Option<DbContact>,
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
            modal_state: ModalState::Off,
            context_menu_position: Offset { x: 0., y: 0. },
            chat_window_size: Size::ZERO,
            chat_total_size: Size::ZERO,
            hide_context_menu: true,
            chat_message_pressed: None,
            last_relays_response: None,
            focus_contact: None,
        }
    }
    pub(crate) fn chat_to(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> (Self, Command<Message>) {
        let mut state = Self::new(conn);
        state.focus_contact = Some(db_contact);
        (state, Command::none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.status_bar
            .subscription()
            .map(Message::StatusBarMessage)
    }

    pub fn view(&self) -> Element<Message> {
        // --- FIRST SPLIT ---
        let contact_list: Element<_> = if self.contacts.is_empty() {
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
                .contacts
                .iter()
                .filter(|c| {
                    contact_matches_search_selected_name(&c.contact, &self.search_contact_input)
                })
                .fold(column![].spacing(0), |col, contact| {
                    col.push(contact.view().map(Message::ContactCardMessage))
                });
            common_scrollable(contact_list)
                .id(CONTACTS_SCROLLABLE_ID.clone())
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

        let second: Element<_> = if let Some(active_contact) = &self.active_contact {
            // Todo: add/remove user button
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
    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        let _command = self.status_bar.backend_event(event.clone(), conn);
        self.modal_state.backend_event(event.clone(), conn);

        match event {
            Event::GotContacts(db_contacts) => {
                self.contacts = db_contacts
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| contact_card::ContactCard::from_db_contact(idx as i32, c, conn))
                    .collect();
                self.sort_contacts();
                if let Some(commands) = self.handle_focus_contact(conn) {
                    return Command::batch(commands);
                }
            }
            Event::GotRelayResponses {
                chat_message,
                responses,
                all_relays,
            } => {
                self.last_relays_response =
                    Some(RelaysResponse::new(chat_message, responses, all_relays));
            }
            Event::GotChatMessages((db_contact, chat_msgs)) => {
                self.update_contact(db_contact.clone(), conn);

                if self.active_contact.as_ref() == Some(&db_contact) {
                    self.messages = chat_msgs;
                    self.messages
                        .sort_by(|a, b| a.display_time.cmp(&b.display_time));

                    self.current_scroll_offset = scrollable::RelativeOffset::END;
                    return scrollable::snap_to(
                        CHAT_SCROLLABLE_ID.clone(),
                        self.current_scroll_offset,
                    );
                }
            }
            Event::ConfirmedDM((_db_contact, chat_msg)) => {
                self.messages
                    .iter_mut()
                    .find(|m| m.msg_id == chat_msg.msg_id)
                    .map(|m| m.confirm_msg(&chat_msg));
            }
            Event::UpdatedContactMetadata { db_contact, .. } => {
                self.update_contact(db_contact, conn);
            }
            Event::PendingDM((_db_contact, msg)) => {
                self.messages.push(msg.clone());
                self.current_scroll_offset = scrollable::RelativeOffset::END;
                return scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.current_scroll_offset);
            }
            Event::ReceivedDM {
                chat_message: msg,
                db_contact,
                ..
            } => {
                self.update_card_last_msg(&db_contact, &msg, conn);
                if self.active_contact.as_ref() == Some(&db_contact) {
                    // estou na conversa
                    self.messages.push(msg.clone());
                    self.current_scroll_offset = scrollable::RelativeOffset::END;
                    return scrollable::snap_to(
                        CHAT_SCROLLABLE_ID.clone(),
                        self.current_scroll_offset,
                    );
                } else {
                    // não estou na conversa
                    self.move_contact_to_top(&db_contact);
                    conn.send(net::Message::AddToUnseenCount(db_contact));
                }
            }
            Event::ContactCreated(db_contact) => {
                self.contacts
                    .push(contact_card::ContactCard::from_db_contact(
                        self.contacts.len() as i32,
                        &db_contact,
                        conn,
                    ));
            }
            Event::GotLastChatMessage((db_contact, chat_message)) => {
                self.update_card_last_msg(&db_contact, &chat_message, conn);
                self.sort_contacts();
            }
            Event::ContactUpdated(db_contact) => {
                self.update_contact(db_contact, conn);
            }
            _ => (),
        };

        Command::none()
    }

    fn handle_focus_contact(
        &mut self,
        conn: &mut BackEndConnection,
    ) -> Option<Vec<Command<Message>>> {
        if let Some(db_contact) = self.focus_contact.take() {
            let mut commands = vec![self.set_active_contact(&db_contact, conn)];

            let list_height: f32 = self.contacts.iter().map(|c| c.height()).sum();
            let position_opt = find_contact_position(&self.contacts, &db_contact);

            // Handle case when position is not found.
            let position = match position_opt {
                Some(pos) => pos,
                None => {
                    // Log an error or handle this situation as needed.
                    return None;
                }
            };

            let card = &mut self.contacts[position];
            card.update(contact_card::Message::TurnOnActive, conn);

            let offset = calculate_scroll_offset(position, list_height, card.height());

            commands.push(scrollable::snap_to(CONTACTS_SCROLLABLE_ID.clone(), offset));

            Some(commands)
        } else {
            None
        }
    }

    fn update_card_last_msg(
        &mut self,
        db_contact: &DbContact,
        chat_message: &ChatMessage,
        conn: &mut BackEndConnection,
    ) {
        if let Some(found_card) = self
            .contacts
            .iter_mut()
            .find(|c| c.contact.pubkey() == db_contact.pubkey())
        {
            found_card.update(
                contact_card::Message::GotLastChatMessage(chat_message.to_owned()),
                conn,
            );
        }
    }

    fn sort_contacts(&mut self) {
        self.contacts
            .sort_by(|a, b| b.contact.select_name().cmp(&a.contact.select_name()));
        self.contacts.sort_by(|a, b| {
            b.contact
                .last_message_date()
                .cmp(&a.contact.last_message_date())
        });
    }
    fn update_contact(&mut self, db_contact: DbContact, conn: &mut BackEndConnection) {
        // change active to be an ID again...
        // println!("Update Contact: {}", &db_contact.pubkey().to_string());
        // dbg!(&db_contact);

        // check if the active is the one who got a new message and update it
        if self.active_contact.as_ref() == Some(&db_contact) {
            self.active_contact = Some(db_contact.clone());
        }

        if let Some(found_card) = self
            .contacts
            .iter_mut()
            .find(|c| c.contact.pubkey() == db_contact.pubkey())
        {
            found_card.update(
                contact_card::Message::ContactUpdated(db_contact.clone()),
                conn,
            );
        } else {
            self.contacts
                .push(contact_card::ContactCard::from_db_contact(
                    self.contacts.len() as i32,
                    &db_contact,
                    conn,
                ));
        }
    }

    fn close_modal(&mut self) -> Command<Message> {
        self.modal_state = ModalState::Off;
        scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.current_scroll_offset)
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        let mut command = Command::none();
        match message {
            Message::CopyPressed => {
                if let Some(chat_msg) = &self.chat_message_pressed {
                    command = clipboard::write(chat_msg.content.to_owned());
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
                command = self.close_modal();
            }
            Message::OpenContactProfile => {
                if let Some(contact) = &self.active_contact {
                    self.modal_state = ModalState::basic_profile(contact, conn);
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
                            command = cmd.map(|m| Message::RelaysConfirmationMessage(Box::new(m)));
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
                            command = cmd.map(|m| Message::BasicContactMessage(Box::new(m)));
                        }
                    }
                }
            }

            Message::StatusBarMessage(status_msg) => {
                let _command = self.status_bar.update(status_msg, conn);
            }
            // ---------
            Message::GoToChannelsPress => (),
            Message::Scrolled(offset) => {
                self.current_scroll_offset = offset;
            }
            Message::GotChatSize((size, child_size)) => {
                self.chat_window_size = size;
                self.chat_total_size = child_size;
            }
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ChatMessage(chat_msg) => match chat_msg {
                chat_message::Message::None => (),
                chat_message::Message::ChatRightClick((msg, point)) => {
                    conn.send(net::Message::FetchRelayResponses(msg.clone()));
                    self.calculate_ctx_menu_pos(point);
                    self.hide_context_menu = false;
                    self.chat_message_pressed = Some(msg);
                }
            },
            Message::AddContactPress => (),
            Message::DMNMessageChange(text) => {
                self.dm_msg = text;
            }
            Message::DMSentPress => match (&self.active_contact, self.dm_msg.is_empty()) {
                (Some(contact), false) => {
                    conn.send(net::Message::SendDM((
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
                        c.update(contact_card::Message::ShowFullCard, conn);
                    }
                } else if position <= PIC_WIDTH {
                    self.ver_divider_position = Some(80);
                    self.show_only_profile = true;
                    for c in &mut self.contacts {
                        c.update(contact_card::Message::ShowOnlyProfileImage, conn);
                    }
                }
            }
            Message::NavSettingsPress => (),
            Message::ContactCardMessage(wrapped) => {
                if let contact_card::Message::ContactPress(contact) = &wrapped.message.clone() {
                    command = self.set_active_contact(contact, conn);
                    self.update_all_contacts(contact_card::Message::TurnOffActive, conn);
                    self.find_and_update_contact(
                        contact_card::Message::TurnOnActive,
                        conn,
                        |contact| contact.id == wrapped.from,
                    );
                }
            }
        }

        command
    }

    fn set_active_contact(
        &mut self,
        contact: &DbContact,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        let mut command = Command::none();
        if self.active_contact.as_ref() != Some(&contact) {
            conn.send(net::Message::FetchMessages(contact.clone()));
            self.messages = vec![];
            command = text_input::focus(CHAT_INPUT_ID.clone());
        }
        self.dm_msg = "".into();
        self.active_contact = Some(contact.clone());
        command
    }

    fn update_all_contacts(
        &mut self,
        message: contact_card::Message,
        conn: &mut BackEndConnection,
    ) {
        for contact in &mut self.contacts {
            contact.update(message.clone(), conn);
        }
    }

    fn find_and_update_contact<F>(
        &mut self,
        message: contact_card::Message,
        conn: &mut BackEndConnection,
        predicate: F,
    ) where
        F: Fn(&&mut ContactCard) -> bool,
    {
        if let Some(contact_card) = self.contacts.iter_mut().find(predicate) {
            contact_card.update(message, conn);
        }
    }

    fn calculate_ctx_menu_pos(&mut self, point: iced_native::Point) {
        let total_h = self.chat_total_size.height;
        let window_h = self.chat_window_size.height;
        let offset_h = self.current_scroll_offset.y;
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
        if let Some(position) = find_contact_position(&self.contacts, db_contact) {
            // If found, remove it from its current position.
            let contact_card = self.contacts.remove(position);
            // Insert it at the top.
            self.contacts.insert(0, contact_card);
        }
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

fn chat_navbar<'a>(active_contact: &'a DbContact) -> Container<'a, Message> {
    container(
        row![header_details(active_contact), header_action_buttons()]
            .spacing(5)
            .width(Length::Fill),
    )
    .height(NAVBAR_HEIGHT)
    .style(style::Container::Default)
}

fn header_details<'a>(db_contact: &'a DbContact) -> Button<'a, Message> {
    let local_message_date = db_contact
        .last_message_date()
        .map(from_naive_utc_to_local)
        .map(|date| format!("last seen {}", date.format(YMD_FORMAT)))
        .unwrap_or("".into());

    let user_name: Element<_> = if let Some(petname) = db_contact.get_petname() {
        if !petname.is_empty() {
            row![
                text(petname).size(20),
                text(db_contact.get_display_name().unwrap_or("".into()))
                    .size(14)
                    .style(style::Text::ChatMessageStatus)
            ]
            .align_items(Alignment::End)
            .spacing(5)
            .into()
        } else {
            text(db_contact.select_name()).size(20).into()
        }
    } else {
        text(db_contact.select_name()).size(20).into()
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
fn find_contact_position(contacts: &[ContactCard], db_contact: &DbContact) -> Option<usize> {
    contacts
        .iter()
        .position(|card| card.contact.pubkey() == db_contact.pubkey())
}

// Function to calculate the scroll offset for a given position
fn calculate_scroll_offset(position: usize, total_height: f32, card_height: f32) -> RelativeOffset {
    let card_list_height = position as f32 * card_height + card_height;
    let y = card_list_height / total_height;
    tracing::info!("position: {}", position);
    tracing::info!("card list height: {}", card_list_height);
    tracing::info!("total_height: {}", total_height);
    tracing::info!("scroll offset: {}", y);
    tracing::info!("-----END----");
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
const MODAL_WIDTH: f32 = 400.0;
const CONTEXT_MENU_WIDTH: f32 = 130.0;
const CTX_BUTTON_HEIGHT: f32 = 30.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_scroll_offset_at_zero() {
        let position = 0;
        let total_height = 200.0;
        let card_height = 20.0;
        let expected_offset = RelativeOffset { x: 0.0, y: 0.0 };
        assert_eq!(
            calculate_scroll_offset(position, total_height, card_height),
            expected_offset
        );
    }

    #[test]
    fn test_calculate_scroll_offset_at_middle() {
        let position = 5;
        let total_height = 200.0;
        let card_height = 20.0;
        let expected_offset = RelativeOffset { x: 0.0, y: 0.5 };
        assert_eq!(
            calculate_scroll_offset(position, total_height, card_height),
            expected_offset
        );
    }

    #[test]
    fn test_calculate_scroll_offset_at_end() {
        let position = 10;
        let total_height = 200.0;
        let card_height = 20.0;
        let expected_offset = RelativeOffset { x: 0.0, y: 1.0 };
        assert_eq!(
            calculate_scroll_offset(position, total_height, card_height),
            expected_offset
        );
    }
}
