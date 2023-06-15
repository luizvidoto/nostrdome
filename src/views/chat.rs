use iced::clipboard;
use iced::subscription::Subscription;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::Size;
use iced::{Alignment, Command, Length};
use iced_native::widget::scrollable::RelativeOffset;
use nostr::secp256k1::XOnlyPublicKey;

use crate::components::chat_contact::{ChatContact, CARD_HEIGHT};
use crate::components::floating_element::{Anchor, FloatingElement, Offset};
use crate::components::{chat_contact, chat_list_container, contact_list};
use crate::db::{DbContact, DbRelay, DbRelayResponse};
use crate::error::BackendClosed;
use crate::icon::{copy_icon, reply_icon, satellite_icon};
use crate::net::{BackEndConnection, BackendEvent, ToBackend};
use crate::style;
use crate::types::ChatMessage;
use crate::widget::Element;
use once_cell::sync::Lazy;

use self::chat_list_container::ChatListContainer;
use self::contact_list::ContactList;

use super::modal::{
    basic_contact, relays_confirmation, ContactDetails, ModalView, RelaysConfirmation,
};
use super::route::Route;
use super::{RouterCommand, RouterMessage};

static CONTACTS_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

// when profile modal is clicked, it sends the scrollable
// to the top but the state thinks that its on the bottom

pub enum ModalState {
    Off,
    BasicProfile(ContactDetails<Message>),
    RelaysConfirmation(RelaysConfirmation<Message>),
}
impl ModalState {
    pub fn basic_profile(
        contact: &DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        Ok(Self::BasicProfile(ContactDetails::viewer(contact, conn)?))
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
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        if let ModalState::BasicProfile(state) = self {
            state.backend_event(event, conn)?
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    CopyPressed,
    ReplyPressed,
    RelaysConfirmationPress,
    BasicContactMessage(Box<basic_contact::CMessage<Message>>),
    RelaysConfirmationMessage(Box<relays_confirmation::CMessage<Message>>),
    OnVerResize(u16),
    CloseModal,
    CloseCtxMenu,
    DebugPressed,

    ContactListMsg(contact_list::Message),
    ChatListContainerMsg(chat_list_container::Message),
}

pub struct State {
    contact_list: ContactList,
    chat_list_container: ChatListContainer,

    ver_divider_position: Option<u16>,
    chats: Vec<ChatContact>,
    active_idx: Option<i32>,
    messages: Vec<ChatMessage>,
    show_only_profile: bool,
    msgs_scroll_offset: scrollable::RelativeOffset,
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
    pub fn new(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchContacts)?;
        Ok(Self {
            contact_list: ContactList::new(),
            chat_list_container: ChatListContainer::new(),

            chats: Vec::new(),
            messages: vec![],
            ver_divider_position: Some(300),
            active_idx: None,
            show_only_profile: false,
            msgs_scroll_offset: scrollable::RelativeOffset::END,
            modal_state: ModalState::Off,
            context_menu_position: Offset { x: 0., y: 0. },
            chat_window_size: Size::ZERO,
            chat_total_size: Size::ZERO,
            hide_context_menu: true,
            chat_message_pressed: None,
            last_relays_response: None,
            focus_pubkey: None,
        })
    }
    pub(crate) fn chat_to(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        let mut state = Self::new(conn)?;
        state.focus_pubkey = Some(db_contact.pubkey().to_owned());
        Ok(state)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
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

    fn handle_focus_contact(
        &mut self,
        conn: &mut BackEndConnection,
    ) -> Result<Option<Vec<Command<Message>>>, BackendClosed> {
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
                    let mut commands = vec![self.set_active_contact(idx, conn)?];
                    let list_height: f32 = self.chats.iter().map(|c| c.height()).sum();
                    let offset = calculate_scroll_offset(position, list_height, CARD_HEIGHT);
                    commands.push(scrollable::snap_to(CONTACTS_SCROLLABLE_ID.clone(), offset));
                    Ok(Some(commands))
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn sort_contacts_name_date(&mut self) {
        self.chats
            .sort_by(|a, b| b.contact.select_name().cmp(&a.contact.select_name()));
        self.chats
            .sort_by(|a, b| b.last_message_date().cmp(&a.last_message_date()));
    }

    fn close_modal(&mut self) -> Command<Message> {
        self.modal_state = ModalState::Off;
        scrollable::snap_to(CHAT_SCROLLABLE_ID.clone(), self.msgs_scroll_offset)
    }

    /// If it finds a contact, focus the text input chat.
    fn set_active_contact(
        &mut self,
        idx: i32,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        if let Some(chat) = self.chats.iter().find(|c| c.id == idx) {
            conn.send(ToBackend::FetchMessages(chat.contact.to_owned()))?;
            self.messages = vec![];
            self.chat_list_container.update_dm_msg("".into());
            self.active_idx = Some(idx);
            return Ok(text_input::focus(CHAT_INPUT_ID.clone()));
        }
        Ok(Command::none())
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

    fn handle_new_message(
        &mut self,
        db_contact: DbContact,
        chat_message: ChatMessage,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        let active_chatting = self.active_matches(&db_contact);

        // push into chat messages
        self.messages.push(chat_message.clone());

        // update chat card headers
        if let Some(contact_card) = self
            .chats
            .iter_mut()
            .find(|c| c.contact.pubkey() == db_contact.pubkey())
        {
            if active_chatting {
                contact_card.update_headers(chat_message);
            } else {
                contact_card.new_message(chat_message);
            }
        } else {
            tracing::info!(
                "New message for a contact not in the list?? {:?}",
                &db_contact
            );
            let new_chat = ChatContact::new(self.chats.len() as i32, &db_contact, conn)?;
            self.chats.push(new_chat);
        }

        self.sort_contacts_name_date();

        // scroll to end
        self.msgs_scroll_offset = scrollable::RelativeOffset::END;
        Ok(scrollable::snap_to(
            CHAT_SCROLLABLE_ID.clone(),
            self.msgs_scroll_offset,
        ))
    }
}

impl Route for State {
    type Message = Message;
    fn view(&self, _selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        // --- FIRST SPLIT ---
        let first_split = self
            .contact_list
            .view(
                &CONTACTS_SCROLLABLE_ID,
                &self.chats,
                self.show_only_profile,
                self.active_idx,
            )
            .map(Message::ContactListMsg);

        // ---
        // --- SECOND SPLIT ---
        let second_split = self
            .chat_list_container
            .view(
                &CHAT_SCROLLABLE_ID,
                &CHAT_INPUT_ID,
                &self.messages,
                self.active_chat(),
            )
            .map(Message::ChatListContainerMsg);

        let main_content = iced_aw::split::Split::new(
            first_split,
            second_split,
            self.ver_divider_position,
            iced_aw::split::Axis::Vertical,
            Message::OnVerResize,
        )
        .spacing(1.0)
        .min_size_second(300);

        let float = FloatingElement::new(main_content, || {
            make_context_menu(&self.last_relays_response)
        })
        .on_esc(Message::CloseCtxMenu)
        .backdrop(Message::CloseCtxMenu)
        .anchor(Anchor::NorthWest)
        .offset(self.context_menu_position)
        .hide(self.hide_context_menu);

        self.modal_state.view(float)
    }

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut commands = RouterCommand::new();

        self.modal_state.backend_event(event.clone(), conn)?;

        match event {
            BackendEvent::ImageDownloaded(image) => {
                if let Some(chat) = self
                    .chats
                    .iter_mut()
                    .find(|c| c.contact.get_profile_event_hash() == Some(image.event_hash))
                {
                    chat.update_image(image);
                }
            }
            BackendEvent::ContactCreated(db_contact) => {
                let id = self.chats.len() as i32;
                let new_chat = chat_contact::ChatContact::new(id, &db_contact, conn)?;
                self.chats.push(new_chat);
                conn.send(ToBackend::FetchContactWithMetadata(
                    db_contact.pubkey().to_owned(),
                ))?;
            }
            BackendEvent::ContactUpdated(db_contact) => {
                if let Some(contact_card) = self
                    .chats
                    .iter_mut()
                    .find(|c| c.contact.pubkey() == db_contact.pubkey())
                {
                    contact_card.update_contact(db_contact, conn)?;
                } else {
                    let new_chat = ChatContact::new(self.chats.len() as i32, &db_contact, conn)?;
                    self.chats.push(new_chat);
                }
            }
            BackendEvent::ContactDeleted(db_contact) => {
                self.chats
                    .retain(|c| c.contact.pubkey() != db_contact.pubkey());
                // Set to none because it can only deletes a contact if the modal is open
                // and if the modal is open, the contact is the active one
                self.active_idx = None;
            }
            BackendEvent::UpdatedMetadata(pubkey) => {
                tracing::info!("Chat got updatedmetadata: {}", pubkey.to_string());
                conn.send(ToBackend::FetchContactWithMetadata(pubkey))?;
            }
            BackendEvent::GotSingleContact(_pubkey, db_contact) => {
                tracing::info!("Chat got single_contact: {:?}", db_contact);
                if let Some(db_contact) = db_contact {
                    if let Some(contact_card) = self
                        .chats
                        .iter_mut()
                        .find(|c| c.contact.pubkey() == db_contact.pubkey())
                    {
                        contact_card.update_contact(db_contact, conn)?;
                    } else {
                        tracing::info!(
                            "GotSingleContact for someone not in the list?? {:?}",
                            db_contact
                        );
                    }
                }
            }
            BackendEvent::GotContacts(db_contacts) => {
                self.chats = vec![];
                for (idx, c) in db_contacts.iter().enumerate() {
                    self.chats
                        .push(chat_contact::ChatContact::new(idx as i32, c, conn)?);
                }

                if let Some(cmds) = self.handle_focus_contact(conn)? {
                    cmds.into_iter().for_each(|c| commands.push(c));
                }
            }
            // instead of fetching relay responses
            // each message already got the responses?
            BackendEvent::GotRelayResponses {
                chat_message,
                responses,
                all_relays,
            } => {
                self.last_relays_response =
                    Some(RelaysResponse::new(chat_message, responses, all_relays));
            }
            BackendEvent::GotChatMessages(db_contact, chat_msgs) => {
                if self.active_matches(&db_contact) {
                    if self.messages.is_empty() {
                        self.messages = chat_msgs;
                        self.msgs_scroll_offset = scrollable::RelativeOffset::END;
                        commands.push(scrollable::snap_to(
                            CHAT_SCROLLABLE_ID.clone(),
                            self.msgs_scroll_offset,
                        ));
                    } else {
                        // TODO: scrollable doesnt stay still when new messages are added at the top
                        self.messages.extend(chat_msgs);
                    }

                    self.messages
                        .sort_by(|a, b| a.display_time.cmp(&b.display_time));
                    self.active_chat_mut().map(|c| c.reset_unseen());
                } else {
                    tracing::info!(
                        "Got chat messages when outside chat?? {:?} - length: {}",
                        db_contact,
                        chat_msgs.len()
                    )
                }
            }
            BackendEvent::ConfirmedDM(chat_message) => {
                if let Some(message) = self
                    .messages
                    .iter_mut()
                    .find(|message| message.msg_id == chat_message.msg_id)
                {
                    message.confirm_msg(&chat_message);
                    conn.send(ToBackend::MessageSeen(message.msg_id))?;
                }
            }
            BackendEvent::PendingDM(db_contact, chat_message)
            | BackendEvent::ReceivedDM {
                chat_message,
                db_contact,
                ..
            } => {
                let cmd = self.handle_new_message(db_contact, chat_message, conn)?;
                commands.push(cmd);
            }

            BackendEvent::GotChatInfo(db_contact, chat_info) => {
                if let Some(contact_card) = self
                    .chats
                    .iter_mut()
                    .find(|c| c.contact.pubkey() == db_contact.pubkey())
                {
                    contact_card.update_chat_info(chat_info);

                    let previous_position = self
                        .chats
                        .iter()
                        .position(|c| c.contact.pubkey() == db_contact.pubkey());

                    self.sort_contacts_name_date();

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
                                let offset = calculate_scroll_offset(
                                    current_position,
                                    list_height,
                                    CARD_HEIGHT,
                                );
                                commands.push(scrollable::snap_to(
                                    CONTACTS_SCROLLABLE_ID.clone(),
                                    offset,
                                ));
                            }
                        }
                    }
                } else {
                    tracing::info!(
                        "Got Chat Info for a contact not in the list?? {:?}",
                        &db_contact
                    );
                }
            }

            _ => (),
        };

        Ok(commands)
    }

    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut commands = RouterCommand::new();

        match message {
            Message::CopyPressed => {
                if let Some(chat_msg) = &self.chat_message_pressed {
                    commands.push(clipboard::write(chat_msg.content.to_owned()));
                }
                self.hide_context_menu = true;
            }
            Message::DebugPressed => {
                if let Some(chat_msg) = &self.chat_message_pressed {
                    tracing::info!("{:?}", chat_msg);
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
            Message::RelaysConfirmationMessage(modal_msg) => {
                if let ModalState::RelaysConfirmation(state) = &mut self.modal_state {
                    match *modal_msg {
                        relays_confirmation::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                commands.push(self.close_modal())
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
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                commands.push(self.close_modal())
                            }
                            commands.push(cmd.map(|m| Message::BasicContactMessage(Box::new(m))));
                        }
                    }
                }
            }
            Message::OnVerResize(position) => {
                if position > 200 && position < 400 {
                    self.ver_divider_position = Some(position);
                    self.show_only_profile = false;
                } else if position <= 200 && position > PIC_WIDTH {
                    self.ver_divider_position = Some(200);
                    self.show_only_profile = false;
                    for c in &mut self.chats {
                        c.big_mode();
                    }
                } else if position <= PIC_WIDTH {
                    self.ver_divider_position = Some(80);
                    self.show_only_profile = true;
                    for c in &mut self.chats {
                        c.small_mode();
                    }
                }
            }

            Message::ChatListContainerMsg(chat_msg) => match chat_msg {
                chat_list_container::Message::DMSentPress(dm_msg) => {
                    match (self.active_chat(), dm_msg.is_empty()) {
                        (Some(chat_contact), false) => {
                            conn.send(ToBackend::SendDM(
                                chat_contact.contact.to_owned(),
                                dm_msg.clone(),
                            ))?;
                            self.chat_list_container.update_dm_msg("".into());
                        }
                        _ => (),
                    }
                }
                chat_list_container::Message::DMNMessageChange(text) => {
                    self.chat_list_container.update_dm_msg(text);
                }
                chat_list_container::Message::GotChatSize(size, child_size) => {
                    self.chat_window_size = size;
                    self.chat_total_size = child_size;
                }
                chat_list_container::Message::Scrolled(offset) => {
                    self.msgs_scroll_offset = offset;

                    // need to see if we need to fetch more messages
                    if self.msgs_scroll_offset.y < 0.01 {
                        if let Some(active_chat) = self.active_chat() {
                            let contact = active_chat.contact.to_owned();
                            if let Some(first_message) = self.messages.first() {
                                tracing::info!(
                                    "fetching more messages, first: {:?}",
                                    first_message
                                );
                                conn.send(ToBackend::FetchMoreMessages(
                                    contact,
                                    first_message.to_owned(),
                                ))?;
                            }
                        }
                    }
                }
                chat_list_container::Message::OpenContactProfile => {
                    if let Some(chat_contact) = self.active_chat() {
                        self.modal_state = ModalState::basic_profile(&chat_contact.contact, conn)?;
                    }
                }
                chat_list_container::Message::ChatRightClick(msg, point) => {
                    conn.send(ToBackend::FetchRelayResponsesChatMsg(msg.clone()))?;
                    self.calculate_ctx_menu_pos(point);
                    self.hide_context_menu = false;
                    self.chat_message_pressed = Some(msg);
                }
                chat_list_container::Message::ChannelMenuPressed => {}
                chat_list_container::Message::ChannelOpenModalPressed => {}
                chat_list_container::Message::ChannelSearchPressed => {}
                chat_list_container::Message::ChannelUserNamePressed(_) => {}
            },

            Message::ContactListMsg(ct_msg) => match ct_msg {
                contact_list::Message::AddContactPress => {
                    commands.change_route(RouterMessage::GoToSettingsContacts);
                }
                contact_list::Message::SearchContactInputChange(text) => {
                    self.contact_list.search_input_change(text);
                }
                contact_list::Message::ContactPress(idx) => {
                    commands.push(self.set_active_contact(idx, conn)?);
                }
            },
        }

        Ok(commands)
    }
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

    // let reply_btn = button(
    //     row![
    //         text("Reply").size(18),
    //         Space::with_width(Length::Fill),
    //         reply_icon().size(16)
    //     ]
    //     .align_items(Alignment::Center),
    // )
    // .height(CTX_BUTTON_HEIGHT)
    // .width(Length::Fill)
    // .on_press(Message::ReplyPressed)
    // .style(style::Button::ContextMenuButton);

    let debug_btn = button(
        row![
            text("Debug").size(18),
            Space::with_width(Length::Fill),
            reply_icon().size(16)
        ]
        .align_items(Alignment::Center),
    )
    .height(CTX_BUTTON_HEIGHT)
    .width(Length::Fill)
    .on_press(Message::DebugPressed)
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

    let buttons = column![debug_btn, copy_btn, relays_btn].spacing(5);

    container(buttons)
        .height(ctx_menu_height())
        .width(CONTEXT_MENU_WIDTH)
        .style(style::Container::ContextMenu)
        .padding(5)
        .into()
}

// Function to calculate the scroll offset for a given position
fn calculate_scroll_offset(position: usize, total_height: f32, card_height: f32) -> RelativeOffset {
    let card_list_height = position as f32 * card_height + card_height;
    let y = card_list_height / total_height;
    RelativeOffset { x: 0.0, y }
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
const CONTEXT_MENU_WIDTH: f32 = 130.0;
const CTX_BUTTON_HEIGHT: f32 = 30.0;
