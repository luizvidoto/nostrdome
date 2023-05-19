use iced::{Command, Subscription};

use crate::{
    components::status_bar,
    db::DbContact,
    net::{
        events::{self, Event},
        BackEndConnection,
    },
    style,
    widget::Element,
};

mod channels;
mod chat;
pub(crate) mod login;
pub(crate) mod modal;
pub(crate) mod settings;
pub(crate) mod welcome;

#[derive(Debug, Clone)]
pub enum RouterMessage {
    GoToSettingsContacts,
    GoToChat,
    GoToChannels,
    GoToAbout,
    GoToNetwork,
    GoToSettings,
    GoToChatTo(DbContact),
    GoBack,
}

#[derive(Debug, Clone)]
pub enum Message {
    ChannelsMsg(channels::Message),
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
}
pub struct Router {
    previous_state: Option<ViewState>,
    state: ViewState,
}
impl Router {
    pub fn new(db_conn: &mut BackEndConnection) -> Self {
        Self {
            previous_state: None,
            state: ViewState::chat(db_conn),
        }
    }
    fn next_state(&mut self, next: ViewState) {
        let old_state = std::mem::replace(&mut self.state, next);
        self.previous_state = Some(old_state);
    }
    fn _next_state_skip(&mut self, next: ViewState) {
        self.state = next;
    }
    fn back(&mut self, conn: &mut BackEndConnection) {
        if let Some(s) = self.previous_state.take() {
            self.state = s;
        } else {
            self.state = Self::new(conn).state;
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.state.subscription()
    }
    pub fn view(&self) -> Element<Message> {
        self.state.view()
    }
    pub fn backend_event(
        &mut self,
        event: events::Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        self.state.backend_event(event, conn)
    }

    pub fn change_route(
        &mut self,
        router_message: RouterMessage,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match router_message {
            RouterMessage::GoBack => self.back(conn),
            RouterMessage::GoToSettingsContacts => {
                self.next_state(ViewState::settings_contacts(conn))
            }
            RouterMessage::GoToChat => self.next_state(ViewState::chat(conn)),
            RouterMessage::GoToChannels => self.next_state(ViewState::channels(conn)),
            RouterMessage::GoToAbout => self.next_state(ViewState::settings_about(conn)),
            RouterMessage::GoToNetwork => self.next_state(ViewState::settings_network(conn)),
            RouterMessage::GoToSettings => self.next_state(ViewState::settings(conn)),
            RouterMessage::GoToChatTo(db_contact) => {
                let (state, command) = ViewState::chat_contact(db_contact, conn);
                self.next_state(state);
                return command;
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
        let (command, router_message) = self.state.update(message, conn, selected_theme);
        if let Some(router_message) = router_message {
            let change_cmd = self.change_route(router_message, conn);
            Command::batch(vec![command, change_cmd])
        } else {
            command
        }
    }
}

pub enum ViewState {
    Channels { state: channels::State },
    Chat { state: chat::State },
    Settings { state: settings::Settings },
}

impl ViewState {
    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            ViewState::Settings { state } => state.subscription().map(Message::SettingsMsg),
            ViewState::Chat { state } => state.subscription().map(Message::ChatMsg),
            _ => Subscription::none(),
        }
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Channels { state } => state.view().map(Message::ChannelsMsg),
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
        }
    }
    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match event {
            event => match self {
                ViewState::Channels { state } => {
                    state.backend_event(event, conn).map(Message::ChannelsMsg)
                }
                ViewState::Chat { state } => state.backend_event(event, conn).map(Message::ChatMsg),
                ViewState::Settings { state } => {
                    state.backend_event(event, conn).map(Message::SettingsMsg)
                }
            },
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> (Command<Message>, Option<RouterMessage>) {
        let mut router_message = None;
        let mut command = Command::none();
        match message {
            Message::ChannelsMsg(msg) => {
                if let ViewState::Channels { state } = self {
                    match msg {
                        channels::Message::GoToChat => {
                            router_message = Some(RouterMessage::GoToChat);
                        }
                        msg => state.update(msg, conn),
                    }
                }
            }
            Message::ChatMsg(msg) => {
                if let ViewState::Chat { state } = self {
                    match msg {
                        chat::Message::AddContactPress => {
                            router_message = Some(RouterMessage::GoToSettingsContacts);
                        }
                        chat::Message::GoToChannelsPress => {
                            router_message = Some(RouterMessage::GoToChannels);
                        }
                        chat::Message::StatusBarMessage(status_msg) => match status_msg {
                            status_bar::Message::GoToAbout => {
                                router_message = Some(RouterMessage::GoToAbout);
                            }
                            status_bar::Message::GoToNetwork => {
                                router_message = Some(RouterMessage::GoToNetwork);
                            }
                            other => {
                                return (
                                    state
                                        .update(chat::Message::StatusBarMessage(other), conn)
                                        .map(Message::ChatMsg),
                                    None,
                                )
                            }
                        },
                        chat::Message::NavSettingsPress => {
                            router_message = Some(RouterMessage::GoToSettings)
                        }
                        msg => return (state.update(msg, conn).map(Message::ChatMsg), None),
                    }
                }
            }
            Message::SettingsMsg(msg) => {
                if let ViewState::Settings { state } = self {
                    match msg {
                        settings::Message::NavEscPress => {
                            router_message = Some(RouterMessage::GoBack)
                        }
                        msg => {
                            let (cmd, msg) = state.update(msg, conn, selected_theme);
                            command = cmd.map(Message::SettingsMsg);
                            router_message = msg;
                        }
                    }
                }
            }
        };
        (command, router_message)
    }

    fn chat_contact(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> (ViewState, Command<Message>) {
        let (state, command) = chat::State::chat_to(db_contact, conn);
        (Self::Chat { state }, command.map(Message::ChatMsg))
    }

    pub fn channels(_db_conn: &mut BackEndConnection) -> ViewState {
        Self::Channels {
            state: channels::State::new(),
        }
    }
    pub fn chat(db_conn: &mut BackEndConnection) -> ViewState {
        Self::Chat {
            state: chat::State::new(db_conn),
        }
    }
    pub fn settings(db_conn: &mut BackEndConnection) -> ViewState {
        Self::Settings {
            state: settings::Settings::new(db_conn),
        }
    }
    pub fn settings_network(db_conn: &mut BackEndConnection) -> ViewState {
        Self::Settings {
            state: settings::Settings::network(db_conn),
        }
    }
    pub fn settings_about(db_conn: &mut BackEndConnection) -> ViewState {
        Self::Settings {
            state: settings::Settings::about(db_conn),
        }
    }
    pub fn settings_contacts(db_conn: &mut BackEndConnection) -> ViewState {
        Self::Settings {
            state: settings::Settings::contacts(db_conn),
        }
    }
}
