use iced::widget::{button, column, container, row, Space};
use iced::{Command, Length, Subscription};

use crate::db::{DbContact, DbRelay};
use crate::error::BackendClosed;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;

use crate::widget::{Button, Element};

use super::modal::{
    basic_contact, import_contact_list, relay_basic, relay_document, relays_confirmation,
    ContactDetails, ImportContactList, ModalView, RelayBasic, RelayDocState, RelaysConfirmation,
};
use super::route::Route;
use super::{RouterCommand, RouterMessage};

mod about;
mod account;
pub mod appearance;
mod backup;
mod contacts;
mod network;

pub enum SettingsRouterMessage {
    OpenRelayBasicModal,
    OpenEditContactModal(DbContact),
    OpenProfileModal(DbContact),
    RouterMessage(RouterMessage),
    OpenImportContactModal,
    OpenAddContactModal,
    OpenRelayDocument(DbRelay),
}

#[derive(Debug, Clone)]
pub enum Message {
    AccountMessage(account::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    ContactsMessage(contacts::Message),
    AboutMessage(about::Message),
    // Modals
    ContactDetailsMessage(Box<basic_contact::CMessage<Message>>),
    ImportContactListMessage(Box<import_contact_list::CMessage<Message>>),
    RelaysConfirmationMessage(Box<relays_confirmation::CMessage<Message>>),
    RelayDocumentMessage(Box<relay_document::CMessage<Message>>),
    RelayBasicMessage(Box<relay_basic::CMessage<Message>>),
    // Navigation
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
    None,
    ChangeTheme(style::Theme),
}

#[repr(u8)]
pub enum MenuState {
    Account { state: account::State } = 0,
    Appearance = 1,
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
            (MenuState::Appearance, Self::APPEARANCE) => true,
            (MenuState::Network { .. }, Self::NETWORK) => true,
            (MenuState::Backup { .. }, Self::BACKUP) => true,
            (MenuState::Contacts { .. }, Self::CONTACTS) => true,
            (MenuState::About { .. }, Self::ABOUT) => true,
            _ => false,
        }
    }
    fn account(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::Account {
            state: account::State::new(conn)?,
        })
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::Network {
            state: network::State::new(db_conn)?,
        })
    }
    pub fn about(_conn: &mut BackEndConnection) -> Self {
        Self::About {
            state: about::State::new(),
        }
    }
    pub fn contacts(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::Contacts {
            state: contacts::State::new(conn)?,
        })
    }
    fn backup(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::Backup {
            state: backup::State::new(conn)?,
        })
    }
    pub fn new(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::account(conn)?)
    }
    pub fn view(&self, selected_theme: Option<style::Theme>) -> Element<Message> {
        match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance => appearance::view(selected_theme).map(|m| match m {
                appearance::Message::ChangeTheme(x) => Message::ChangeTheme(x),
            }),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
            Self::Contacts { state } => state.view().map(Message::ContactsMessage),
            Self::About { state } => state.view().map(Message::AboutMessage),
        }
    }
}

pub struct Settings {
    menu_state: MenuState,
    modal_state: ModalState,
}
impl Settings {
    fn with_menu_state(menu_state: MenuState) -> Self {
        Self {
            menu_state,
            modal_state: ModalState::Off,
        }
    }
    pub fn new(db_conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::with_menu_state(MenuState::new(db_conn)?))
    }
    pub fn contacts(db_conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::with_menu_state(MenuState::contacts(db_conn)?))
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        Ok(Self::with_menu_state(MenuState::network(db_conn)?))
    }
    pub fn about(db_conn: &mut BackEndConnection) -> Self {
        Self::with_menu_state(MenuState::about(db_conn))
    }
    fn handle_menu_press(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        match message {
            Message::MenuAccountPress => match self.menu_state {
                MenuState::Account { .. } => (),
                _ => self.menu_state = MenuState::account(conn)?,
            },
            Message::MenuAppearancePress => match self.menu_state {
                MenuState::Appearance { .. } => (),
                _ => self.menu_state = MenuState::Appearance,
            },
            Message::MenuNetworkPress => match self.menu_state {
                MenuState::Network { .. } => (),
                _ => self.menu_state = MenuState::network(conn)?,
            },
            Message::MenuBackupPress => match self.menu_state {
                MenuState::Backup { .. } => (),
                _ => self.menu_state = MenuState::backup(conn)?,
            },
            Message::MenuContactsPress => match self.menu_state {
                MenuState::Contacts { .. } => (),
                _ => self.menu_state = MenuState::contacts(conn)?,
            },
            Message::MenuAboutPress => match self.menu_state {
                MenuState::About { .. } => (),
                _ => self.menu_state = MenuState::about(conn),
            },
            _ => (),
        }
        Ok(())
    }
    fn handle_settings_route_msg(
        &mut self,
        message: SettingsRouterMessage,
        conn: &mut BackEndConnection,
    ) -> Result<Option<RouterMessage>, BackendClosed> {
        let mut router_message = None;
        match message {
            SettingsRouterMessage::OpenRelayBasicModal => {
                self.modal_state = ModalState::RelayBasic(RelayBasic::new());
            }
            SettingsRouterMessage::OpenRelayDocument(db_relay) => {
                self.modal_state = ModalState::RelayDocument(RelayDocState::new(db_relay, conn)?);
            }
            SettingsRouterMessage::OpenAddContactModal => {
                self.modal_state = ModalState::ContactDetails(ContactDetails::new());
            }
            SettingsRouterMessage::OpenImportContactModal => {
                self.modal_state = ModalState::import_contacts();
            }
            SettingsRouterMessage::OpenEditContactModal(contact) => {
                self.modal_state = ModalState::ContactDetails(ContactDetails::edit(&contact, conn)?)
            }
            SettingsRouterMessage::OpenProfileModal(contact) => {
                self.modal_state =
                    ModalState::ContactDetails(ContactDetails::viewer(&contact, conn)?)
            }
            SettingsRouterMessage::RouterMessage(router_msg) => {
                router_message = Some(router_msg);
            }
        }
        Ok(router_message)
    }

    fn handle_contacts_message(
        &mut self,
        msg: contacts::Message,
        conn: &mut BackEndConnection,
    ) -> Result<Option<RouterMessage>, BackendClosed> {
        let mut router_message = None;

        if let MenuState::Contacts { state } = &mut self.menu_state {
            match msg {
                contacts::Message::RelaysConfirmationPress(ct_resp) => {
                    if let Some(ct_resp) = ct_resp {
                        self.modal_state = ModalState::RelaysConfirmation(RelaysConfirmation::new(
                            &ct_resp.confirmed_relays,
                            &ct_resp.all_relays,
                        ))
                    }
                }
                other => {
                    if let Some(received_msg) = state.update(other, conn)? {
                        router_message = self.handle_settings_route_msg(received_msg, conn)?;
                    }
                }
            }
        }

        Ok(router_message)
    }
}

impl Route for Settings {
    type Message = Message;
    fn subscription(&self) -> Subscription<Self::Message> {
        let menu_subs = match &self.menu_state {
            MenuState::Network { state } => state.subscription().map(Message::NetworkMessage),
            _ => Subscription::none(),
        };
        Subscription::batch(vec![menu_subs, self.modal_state.subscription()])
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let commands = RouterCommand::new();

        self.modal_state.backend_event(event.clone(), conn)?;

        match &mut self.menu_state {
            MenuState::About { state } => {
                state.backend_event(event, conn);
            }
            MenuState::Account { state } => {
                state.backend_event(event, conn)?;
            }
            MenuState::Appearance => {}
            MenuState::Network { state } => {
                state.backend_event(event, conn);
            }
            MenuState::Backup { state } => {
                state.backend_event(event, conn);
            }
            MenuState::Contacts { state } => {
                state.backend_event(event, conn)?;
            }
        }

        Ok(commands)
    }
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut commands = RouterCommand::new();

        match message {
            Message::ChangeTheme(theme) => {
                conn.send(net::ToBackend::SetTheme(theme))?;
            }
            Message::AccountMessage(msg) => {
                if let MenuState::Account { state } = &mut self.menu_state {
                    match msg {
                        account::Message::RelaysConfirmationPress(acc_resp) => {
                            if let Some(acc_resp) = acc_resp {
                                self.modal_state =
                                    ModalState::RelaysConfirmation(RelaysConfirmation::new(
                                        &acc_resp.confirmed_relays,
                                        &acc_resp.all_relays,
                                    ))
                            }
                        }
                        other => {
                            state.update(other, conn)?;
                        }
                    }
                }
            }
            Message::AboutMessage(msg) => {
                if let MenuState::About { state } = &mut self.menu_state {
                    let cmd = state.update(msg);
                    commands.push(cmd.map(Message::AboutMessage))
                }
            }
            Message::NetworkMessage(msg) => {
                if let MenuState::Network { state } = &mut self.menu_state {
                    if let Some(received_msg) = state.update(msg, conn)? {
                        if let Some(router_message) =
                            self.handle_settings_route_msg(received_msg, conn)?
                        {
                            commands.change_route(router_message);
                        }
                    }
                }
            }
            Message::BackupMessage(msg) => {
                if let MenuState::Backup { state } = &mut self.menu_state {
                    let cmd = state.update(msg, conn)?;
                    commands.push(cmd.map(Message::BackupMessage));
                }
            }
            Message::ContactsMessage(msg) => {
                if let Some(router_message) = self.handle_contacts_message(msg, conn)? {
                    commands.change_route(router_message);
                }
            }
            Message::NavEscPress => commands.change_route(RouterMessage::GoToChat),
            Message::MenuAccountPress
            | Message::MenuAppearancePress
            | Message::MenuNetworkPress
            | Message::MenuBackupPress
            | Message::MenuContactsPress
            | Message::MenuAboutPress => {
                self.handle_menu_press(message, conn)?;
            }
            Message::LogoutPress => {
                conn.send(net::ToBackend::Logout)?;
                commands.change_route(RouterMessage::GoToLogout)
            }
            other => {
                let cmd = self.modal_state.update(other, conn)?;
                commands.push(cmd);
            }
        }

        Ok(commands)
    }

    fn view(&self, selected_theme: Option<style::Theme>) -> Element<Self::Message> {
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
        .style(style::Container::Frame)
        .height(Length::Fill)
        .width(Length::Fixed(MENU_WIDTH))
        .padding([10, 5]);

        let view_ct = container(self.menu_state.view(selected_theme))
            .padding([0, 0, 0, 20])
            .height(Length::Fill)
            .width(Length::FillPortion(3));

        let underlay = container(
            row![menubar, view_ct]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        self.modal_state.view(underlay)
    }
}

enum ModalState {
    RelaysConfirmation(RelaysConfirmation<Message>),
    ContactDetails(ContactDetails<Message>),
    ImportList(ImportContactList<Message>),
    RelayDocument(RelayDocState<Message>),
    RelayBasic(RelayBasic<Message>),
    Off,
}

impl ModalState {
    pub fn import_contacts() -> Self {
        Self::ImportList(ImportContactList::new())
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Message>, BackendClosed> {
        let command = RouterCommand::new();
        match self {
            ModalState::RelayDocument(state) => {
                state.backend_event(event, conn)?;
            }
            ModalState::ContactDetails(state) => {
                state.backend_event(event, conn)?;
            }
            ModalState::RelaysConfirmation(state) => {
                state.backend_event(event, conn)?;
            }
            ModalState::ImportList(state) => {
                state.backend_event(event, conn)?;
            }
            ModalState::RelayBasic(state) => {
                state.backend_event(event, conn)?;
            }
            ModalState::Off => (),
        }
        Ok(command)
    }
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        let mut command = Command::none();

        match message {
            Message::CloseModal => *self = Self::Off,
            Message::SendContactListToAll => {
                println!("Send contacts to all relays");
            }
            Message::RelayBasicMessage(modal_msg) => {
                if let ModalState::RelayBasic(state) = self {
                    match *modal_msg {
                        relay_basic::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                *self = ModalState::Off;
                            }
                            command = cmd.map(|m| Message::RelayBasicMessage(Box::new(m)));
                        }
                    }
                }
            }
            Message::RelaysConfirmationMessage(modal_msg) => {
                if let ModalState::RelaysConfirmation(state) = self {
                    match *modal_msg {
                        relays_confirmation::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                *self = ModalState::Off;
                            }
                            command = cmd.map(|m| Message::RelaysConfirmationMessage(Box::new(m)));
                        }
                    }
                }
            }
            Message::ContactDetailsMessage(modal_msg) => {
                if let ModalState::ContactDetails(state) = self {
                    match *modal_msg {
                        basic_contact::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                *self = ModalState::Off;
                            }
                            command = cmd.map(|m| Message::ContactDetailsMessage(Box::new(m)));
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
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                *self = ModalState::Off
                            }
                            command = cmd.map(|m| Message::ImportContactListMessage(Box::new(m)));
                        }
                    }
                }
            }
            Message::RelayDocumentMessage(modal_msg) => {
                if let ModalState::RelayDocument(state) = self {
                    match *modal_msg {
                        relay_document::CMessage::UnderlayMessage(message) => {
                            return self.update(message, conn);
                        }
                        other => {
                            let (cmd, close_modal) = state.update(other, conn)?;
                            if close_modal {
                                *self = ModalState::Off
                            }
                            command = cmd.map(|m| Message::RelayDocumentMessage(Box::new(m)));
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(command)
    }

    fn view<'a>(&'a self, underlay: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        let view: Element<_> = match self {
            ModalState::RelayBasic(state) => state
                .view(underlay)
                .map(|m| Message::RelayBasicMessage(Box::new(m))),
            ModalState::RelayDocument(state) => state
                .view(underlay)
                .map(|m| Message::RelayDocumentMessage(Box::new(m))),
            ModalState::RelaysConfirmation(state) => state
                .view(underlay)
                .map(|m| Message::RelaysConfirmationMessage(Box::new(m))),
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
