use iced::widget::{button, column, container, row, Space};
use iced::{Command, Length, Subscription};

use crate::components::relay_row;
use crate::db::DbContact;
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;

use crate::widget::{Button, Element};

use super::modal::send_contact_list::SendContactList;
use super::modal::{
    basic_contact, import_contact_list, profile_view, send_contact_list, ContactDetails,
    ImportContactList, ProfileView,
};

mod about;
mod account;
pub mod appearance;
mod backup;
mod contacts;
mod network;

#[derive(Debug, Clone)]
pub enum Message {
    AccountMessage(account::Message),
    AppearanceMessage(appearance::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    ContactsMessage(contacts::Message),
    AboutMessage(about::Message),
    // Modals
    ContactDetailsMessage(Box<basic_contact::CMessage<Message>>),
    ImportContactListMessage(Box<import_contact_list::CMessage<Message>>),
    ProfileViewMessage(Box<profile_view::CMessage<Message>>),
    SendContactListMessage(Box<send_contact_list::CMessage<Message>>),
    // Menu
    MenuAccountPress,
    MenuAppearancePress,
    MenuNetworkPress,
    MenuBackupPress,
    MenuContactsPress,
    MenuAboutPress,
    LogoutPress,
    NavEscPress,
    // Modal Messages
    SendContactListToAll,
    CloseModal,
    // Other
    UpdateSelf(Box<Message>),
    UpdateSendContactList(send_contact_list::CMessage<Box<Message>>),
    None,
}

#[derive(Debug)]
#[repr(u8)]
pub enum MenuState {
    Account { state: account::State } = 0,
    Appearance { state: appearance::State } = 1,
    Network { state: network::State } = 2,
    Backup { state: backup::State } = 3,
    Contacts { state: contacts::State } = 4,
    About { state: about::State } = 10,
}

impl MenuState {
    const ACCOUNT: u8 = 0;
    const APPEARANCE: u8 = 1;
    const NETWORK: u8 = 2;
    const BACKUP: u8 = 3;
    const CONTACTS: u8 = 4;
    const ABOUT: u8 = 10;

    pub fn is_same_type(&self, other: u8) -> bool {
        match (self, other) {
            (MenuState::Account { .. }, Self::ACCOUNT) => true,
            (MenuState::Appearance { .. }, Self::APPEARANCE) => true,
            (MenuState::Network { .. }, Self::NETWORK) => true,
            (MenuState::Backup { .. }, Self::BACKUP) => true,
            (MenuState::Contacts { .. }, Self::CONTACTS) => true,
            (MenuState::About { .. }, Self::ABOUT) => true,
            _ => false,
        }
    }
    fn account(conn: &mut BackEndConnection) -> Self {
        Self::Account {
            state: account::State::new(conn),
        }
    }
    fn appearance(selected_theme: Option<style::Theme>) -> Self {
        Self::Appearance {
            state: appearance::State::new(selected_theme),
        }
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Self {
        Self::Network {
            state: network::State::new(db_conn),
        }
    }
    pub fn about(_conn: &mut BackEndConnection) -> Self {
        Self::About {
            state: about::State::new(),
        }
    }
    pub fn contacts(conn: &mut BackEndConnection) -> Self {
        Self::Contacts {
            state: contacts::State::new(conn),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn new(conn: &mut BackEndConnection) -> Self {
        Self::account(conn)
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance { state } => state.view().map(Message::AppearanceMessage),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
            Self::Contacts { state } => state.view().map(Message::ContactsMessage),
            Self::About { state } => state.view().map(Message::AboutMessage),
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    menu_state: MenuState,
    modal_state: ModalState,
}
impl Settings {
    pub fn subscription(&self) -> Subscription<Message> {
        let menu_subs = match &self.menu_state {
            MenuState::Network { state } => state.subscription().map(Message::NetworkMessage),
            _ => Subscription::none(),
        };
        Subscription::batch(vec![menu_subs, self.modal_state.subscription()])
    }
    fn with_menu_state(menu_state: MenuState) -> Self {
        Self {
            menu_state,
            modal_state: ModalState::Off,
        }
    }
    pub fn new(db_conn: &mut BackEndConnection) -> Self {
        Self::with_menu_state(MenuState::new(db_conn))
    }
    pub fn contacts(db_conn: &mut BackEndConnection) -> Self {
        Self::with_menu_state(MenuState::contacts(db_conn))
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Self {
        Self::with_menu_state(MenuState::network(db_conn))
    }
    pub fn about(db_conn: &mut BackEndConnection) -> Self {
        Self::with_menu_state(MenuState::about(db_conn))
    }

    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        self.modal_state.backend_event(event.clone(), conn);

        match &mut self.menu_state {
            MenuState::About { state } => state.update(about::Message::BackEndEvent(event)),
            MenuState::Account { state } => {
                state.update(account::Message::BackEndEvent(event), conn)
            }
            MenuState::Appearance { state } => {
                state.update(appearance::Message::BackEndEvent(event))
            }
            MenuState::Network { state } => {
                return state
                    .update(network::Message::BackEndEvent(event), conn)
                    .map(Message::NetworkMessage);
            }
            MenuState::Backup { state } => state.update(backup::Message::BackEndEvent(event)),
            MenuState::Contacts { state } => {
                let _ = state.update(contacts::Message::BackEndEvent(event), conn);
            }
        }

        Command::none()
    }
    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> Command<Message> {
        match message {
            Message::AccountMessage(msg) => {
                if let MenuState::Account { state } = &mut self.menu_state {
                    state.update(msg, conn);
                }
            }
            Message::AppearanceMessage(msg) => {
                if let MenuState::Appearance { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::NetworkMessage(msg) => {
                if let MenuState::Network { state } = &mut self.menu_state {
                    return state.update(msg, conn).map(Message::NetworkMessage);
                }
            }
            Message::BackupMessage(msg) => {
                if let MenuState::Backup { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::ContactsMessage(msg) => self.handle_contacts_message(msg, conn),
            Message::NavEscPress => (),
            Message::MenuAccountPress
            | Message::MenuAppearancePress
            | Message::MenuNetworkPress
            | Message::MenuBackupPress
            | Message::MenuContactsPress
            | Message::MenuAboutPress => {
                self.handle_menu_press(message, conn, selected_theme);
            }
            Message::LogoutPress => {
                conn.send(net::Message::Logout);
            }
            other => return self.modal_state.update(other, conn),
        }
        Command::none()
    }

    fn handle_menu_press(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) {
        match message {
            Message::MenuAccountPress => self.menu_state = MenuState::account(conn),
            Message::MenuAppearancePress => self.menu_state = MenuState::appearance(selected_theme),
            Message::MenuNetworkPress => self.menu_state = MenuState::network(conn),
            Message::MenuBackupPress => self.menu_state = MenuState::backup(),
            Message::MenuContactsPress => self.menu_state = MenuState::contacts(conn),
            Message::MenuAboutPress => self.menu_state = MenuState::about(conn),
            _ => (),
        }
    }

    fn handle_contacts_message(&mut self, msg: contacts::Message, conn: &mut BackEndConnection) {
        // TODO: make it better
        if let MenuState::Contacts { state } = &mut self.menu_state {
            match msg {
                contacts::Message::OpenAddContactModal => {
                    self.modal_state = ModalState::add_contact(None);
                }
                contacts::Message::OpenImportContactModal => {
                    self.modal_state = ModalState::import_contacts();
                }
                contacts::Message::OpenSendContactModal => {
                    self.modal_state = ModalState::load_send_contacts(conn);
                }
                other => {
                    if let Some(received_msg) = state.update(other, conn) {
                        match received_msg {
                            contacts::Message::OpenEditContactModal(contact) => {
                                self.modal_state = ModalState::add_contact(Some(contact))
                            }
                            contacts::Message::OpenProfileModal(contact) => {
                                self.modal_state = ModalState::profile(contact);
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let account_btn =
            create_menu_button("Account", &self.menu_state, 0, Message::MenuAccountPress);
        let appearance_btn = create_menu_button(
            "Appearance",
            &self.menu_state,
            1,
            Message::MenuAppearancePress,
        );
        let network_btn =
            create_menu_button("Network", &self.menu_state, 2, Message::MenuNetworkPress);
        let backup_btn =
            create_menu_button("Backup", &self.menu_state, 3, Message::MenuBackupPress);
        let contacts_btn =
            create_menu_button("Contacts", &self.menu_state, 4, Message::MenuContactsPress);
        let about_btn = create_menu_button("About", &self.menu_state, 10, Message::MenuAboutPress);
        let logout_btn = button("Logout")
            .padding(10)
            .on_press(Message::LogoutPress)
            .style(style::Button::MenuBtn);
        let esc_btn = button("Esc")
            .padding(10)
            .on_press(Message::NavEscPress)
            .style(style::Button::MenuBtn);
        let spacer = iced::widget::horizontal_space(Length::Fixed(3.0));
        let menubar = container(
            column![
                esc_btn,
                spacer,
                account_btn,
                appearance_btn,
                network_btn,
                backup_btn,
                contacts_btn,
                about_btn,
                Space::with_height(Length::Fill),
                logout_btn
            ]
            .spacing(3),
        )
        .height(Length::Fill)
        .width(Length::Fixed(MENU_WIDTH))
        .padding([10, 5]);

        let view_ct = container(self.menu_state.view())
            .style(style::Container::ChatContainer)
            .padding([20, 20, 0, 20])
            .height(Length::Fill)
            .width(Length::FillPortion(3));

        let underlay = container(
            row![menubar, view_ct]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(style::Container::Default)
        .width(Length::Fill)
        .height(Length::Fill);

        self.modal_state.view(underlay)
    }
}

#[derive(Debug, Clone)]
enum ModalState {
    Profile(ProfileView),
    ContactDetails(ContactDetails),
    ImportList(ImportContactList),
    SendContactList(SendContactList),
    Off,
}

impl ModalState {
    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            ModalState::SendContactList(state) => state.subscription().map(|msg| match msg {
                send_contact_list::CMessage::UnderlayMessage(message) => {
                    Message::UpdateSelf(message)
                }
                other => Message::UpdateSendContactList(other),
            }),
            _ => Subscription::none(),
        }
    }
    pub fn backend_event(&mut self, event: Event, conn: &mut BackEndConnection) {
        if let ModalState::SendContactList(state) = self {
            state.backend_event(event, conn);
        }
    }
    fn profile(db_contact: DbContact) -> Self {
        Self::Profile(ProfileView::new(db_contact))
    }
    pub fn load_send_contacts(conn: &mut BackEndConnection) -> Self {
        Self::SendContactList(SendContactList::new(conn))
    }
    pub fn add_contact(db_contact: Option<DbContact>) -> Self {
        Self::ContactDetails(ContactDetails::new(db_contact))
    }
    pub fn import_contacts() -> Self {
        Self::ImportList(ImportContactList::new())
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        match message {
            Message::UpdateSelf(self_msg) => {
                return self.update(*self_msg, conn);
            }
            Message::CloseModal => *self = Self::Off,
            Message::SendContactListToAll => {
                println!("Send contacts to all relays");
            }

            Message::ContactDetailsMessage(modal_msg) => {
                if let ModalState::ContactDetails(state) = self {
                    match *modal_msg {
                        basic_contact::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let close_modal = state.update(other, conn);
                            if close_modal {
                                *self = ModalState::Off
                            }
                        }
                    }
                }
            }
            Message::ImportContactListMessage(modal_msg) => {
                if let ModalState::ImportList(state) = self {
                    match *modal_msg {
                        import_contact_list::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let close_modal = state.update(other, conn);
                            if close_modal {
                                *self = ModalState::Off
                            }
                        }
                    }
                }
            }
            Message::ProfileViewMessage(modal_msg) => {
                if let ModalState::Profile(state) = self {
                    match *modal_msg {
                        profile_view::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let close_modal = state.update(other, conn);
                            if close_modal {
                                *self = ModalState::Off
                            }
                        }
                    }
                }
            }
            Message::UpdateSendContactList(other_msg) => {
                if let ModalState::SendContactList(state) = self {
                    let close_modal = state.update(other_msg, conn);
                    if close_modal {
                        *self = ModalState::Off
                    }
                }
            }
            Message::SendContactListMessage(modal_msg) => {
                if let ModalState::SendContactList(state) = self {
                    match *modal_msg {
                        send_contact_list::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let close_modal = state.update(other, conn);
                            if close_modal {
                                *self = ModalState::Off
                            }
                        }
                    }
                }
            }
            _ => (),
        }

        Command::none()
    }

    pub fn view<'a>(&'a self, underlay: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        let view: Element<_> = match self {
            ModalState::Profile(state) => state
                .view(underlay)
                .map(|m| Message::ProfileViewMessage(Box::new(m))),
            ModalState::SendContactList(state) => state
                .view(underlay)
                .map(|m| Message::SendContactListMessage(Box::new(m))),
            ModalState::ContactDetails(state) => state
                .view(underlay)
                .map(|m| Message::ContactDetailsMessage(Box::new(m))),
            ModalState::ImportList(state) => state
                .view(underlay)
                .map(|m| Message::ImportContactListMessage(Box::new(m))),
            ModalState::Off => underlay.into(),
        };

        view
    }
}

fn create_menu_button<'a>(
    label: &'a str,
    current_menu_state: &MenuState,
    target_menu_type: u8,
    msg: Message,
) -> Button<'a, Message> {
    let is_active = current_menu_state.is_same_type(target_menu_type);

    let style = if is_active {
        style::Button::ActiveMenuBtn
    } else {
        style::Button::MenuBtn
    };

    button(label)
        .style(style)
        .width(Length::Fill)
        .padding(10)
        .on_press(msg)
}

const MENU_WIDTH: f32 = 150.0;
