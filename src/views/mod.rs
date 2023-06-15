use iced::{Command, Subscription};

use crate::{
    db::DbContact,
    error::BackendClosed,
    net::{BackEndConnection, BackendEvent},
    style,
    widget::Element,
};

use self::route::Route;

mod channel;
mod chat;
mod color_palettes;
mod find_channels;
pub(crate) mod home;
pub(crate) mod login;
mod logout;
pub(crate) mod modal;
mod route;
pub(crate) mod settings;
pub(crate) mod welcome;

pub struct RouterCommand<M> {
    commands: Vec<Command<M>>,
    router_message: Option<RouterMessage>,
}
impl<M> RouterCommand<M> {
    pub fn new() -> Self {
        Self {
            commands: vec![],
            router_message: None,
        }
    }
    pub fn push(&mut self, command: Command<M>) {
        self.commands.push(command);
    }
    pub fn change_route(&mut self, router_message: RouterMessage) {
        self.router_message = Some(router_message);
    }
    pub fn batch(self) -> (Command<M>, Option<RouterMessage>) {
        (Command::batch(self.commands), self.router_message)
    }

    pub fn map<F, N>(self, f: F) -> RouterCommand<N>
    where
        F: Fn(M) -> N + 'static + Clone + Send + Sync,
        M: 'static,
        N: 'static,
    {
        let new_commands = self
            .commands
            .into_iter()
            .map(|command| command.map(f.clone()))
            .collect();
        RouterCommand {
            commands: new_commands,
            router_message: self.router_message,
        }
    }
}

pub enum RouterMessage {
    GoToSettingsContacts,
    GoToChat,
    GoToChannels,
    GoToAbout,
    GoToNetwork,
    GoToSettings,
    GoToChatTo(DbContact),
    GoToWelcome,
    GoToLogin,
    GoToLogout,
    GoBack,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeMsg(home::Message),
    SettingsMsg(settings::Message),
    LoginMsg(login::Message),
    LogoutMsg(logout::Message),
    WelcomeMsg(welcome::Message),
}
pub struct Router {
    previous_state: Option<ViewState>,
    state: ViewState,
}
impl Router {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        let (state, _command) = ViewState::login(conn);
        Self {
            previous_state: None,
            state,
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
    pub fn view(&self, selected_theme: Option<style::Theme>) -> Element<Message> {
        self.state.view(selected_theme)
    }

    pub fn change_route(
        &mut self,
        router_message: RouterMessage,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        match router_message {
            RouterMessage::GoToLogout => self.next_state(ViewState::Logout {
                state: logout::State::new(),
            }),
            RouterMessage::GoBack => self.back(conn),
            RouterMessage::GoToSettingsContacts => {
                self.next_state(ViewState::settings_contacts(conn)?)
            }
            RouterMessage::GoToChat => self.next_state(ViewState::chat(conn)?),
            RouterMessage::GoToChannels => self.next_state(ViewState::channels(conn)?),
            RouterMessage::GoToAbout => self.next_state(ViewState::settings_about(conn)),
            RouterMessage::GoToNetwork => self.next_state(ViewState::settings_network(conn)?),
            RouterMessage::GoToSettings => self.next_state(ViewState::settings(conn)?),
            RouterMessage::GoToChatTo(db_contact) => {
                let state = ViewState::chat_contact(db_contact, conn)?;
                self.next_state(state);
            }
            RouterMessage::GoToLogin => {
                let (state, command) = ViewState::login(conn);
                self.next_state(state);
                return Ok(command);
            }
            RouterMessage::GoToWelcome => {
                let (state, command) = ViewState::welcome(conn);
                self.next_state(state);
                return Ok(command);
            }
        }

        Ok(Command::none())
    }

    pub fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        let (command, router_message) = self.state.backend_event(event, conn)?.batch();
        if let Some(router_message) = router_message {
            let change_cmd = self.change_route(router_message, conn)?;
            Ok(Command::batch(vec![command, change_cmd]))
        } else {
            Ok(command)
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        let (command, router_message) = self.state.update(message, conn)?.batch();
        if let Some(router_message) = router_message {
            let change_cmd = self.change_route(router_message, conn)?;
            Ok(Command::batch(vec![command, change_cmd]))
        } else {
            Ok(command)
        }
    }
}

pub enum ViewState {
    Welcome { state: welcome::State },
    Home { state: home::State },
    Login { state: login::State },
    Logout { state: logout::State },
    Settings { state: settings::Settings },
}

impl ViewState {
    fn login(_conn: &mut BackEndConnection) -> (ViewState, Command<Message>) {
        let state = login::State::new();
        (Self::Login { state }, Command::none())
    }

    fn welcome(_conn: &mut BackEndConnection) -> (ViewState, Command<Message>) {
        let state = welcome::State::new();
        (Self::Welcome { state }, Command::none())
    }

    fn chat_contact(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<ViewState, BackendClosed> {
        Ok(Self::Home {
            state: home::State::chat_to(db_contact, conn)?,
        })
    }
    pub fn chat(conn: &mut BackEndConnection) -> Result<ViewState, BackendClosed> {
        Ok(Self::Home {
            state: home::State::chat(conn)?,
        })
    }
    pub fn channels(conn: &mut BackEndConnection) -> Result<ViewState, BackendClosed> {
        Ok(Self::Home {
            state: home::State::find_channels(conn)?,
        })
    }
    pub fn settings(conn: &mut BackEndConnection) -> Result<ViewState, BackendClosed> {
        Ok(Self::Settings {
            state: settings::Settings::new(conn)?,
        })
    }
    pub fn settings_network(conn: &mut BackEndConnection) -> Result<ViewState, BackendClosed> {
        Ok(Self::Settings {
            state: settings::Settings::network(conn)?,
        })
    }
    pub fn settings_about(conn: &mut BackEndConnection) -> ViewState {
        Self::Settings {
            state: settings::Settings::about(conn),
        }
    }
    pub fn settings_contacts(conn: &mut BackEndConnection) -> Result<ViewState, BackendClosed> {
        Ok(Self::Settings {
            state: settings::Settings::contacts(conn)?,
        })
    }
}

impl Route for ViewState {
    type Message = Message;
    fn subscription(&self) -> Subscription<Self::Message> {
        match self {
            ViewState::Settings { state } => state.subscription().map(Message::SettingsMsg),
            ViewState::Home { state } => state.subscription().map(Message::HomeMsg),
            ViewState::Welcome { state } => state.subscription().map(Message::WelcomeMsg),
            _ => Subscription::none(),
        }
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<'_, Self::Message> {
        match self {
            Self::Welcome { state } => state.view(selected_theme).map(Message::WelcomeMsg),
            Self::Home { state } => state.view(selected_theme).map(Message::HomeMsg),
            Self::Login { state } => state.view(selected_theme).map(Message::LoginMsg),
            Self::Logout { state } => state.view(selected_theme).map(Message::LogoutMsg),
            Self::Settings { state } => state.view(selected_theme).map(Message::SettingsMsg),
        }
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let command = match self {
            ViewState::Logout { state } => {
                state.backend_event(event, conn)?.map(Message::LogoutMsg)
            }
            ViewState::Home { state } => state.backend_event(event, conn)?.map(Message::HomeMsg),
            ViewState::Settings { state } => {
                state.backend_event(event, conn)?.map(Message::SettingsMsg)
            }
            ViewState::Welcome { state } => {
                state.backend_event(event, conn)?.map(Message::WelcomeMsg)
            }
            ViewState::Login { state } => state.backend_event(event, conn)?.map(Message::LoginMsg),
        };

        Ok(command)
    }
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let command = 'command: {
            match self {
                Self::Welcome { state } => {
                    if let Message::WelcomeMsg(msg) = message {
                        break 'command state.update(msg, conn)?.map(Message::WelcomeMsg);
                    }
                }
                Self::Home { state } => {
                    if let Message::HomeMsg(msg) = message {
                        break 'command state.update(msg, conn)?.map(Message::HomeMsg);
                    }
                }
                Self::Login { state } => {
                    if let Message::LoginMsg(msg) = message {
                        break 'command state.update(msg, conn)?.map(Message::LoginMsg);
                    }
                }
                Self::Logout { state } => {
                    if let Message::LogoutMsg(msg) = message {
                        break 'command state.update(msg, conn)?.map(Message::LogoutMsg);
                    }
                }
                Self::Settings { state } => {
                    if let Message::SettingsMsg(msg) = message {
                        break 'command state.update(msg, conn)?.map(Message::SettingsMsg);
                    }
                }
            };
            RouterCommand::new()
        };

        Ok(command)
    }
}
