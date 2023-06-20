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
    router_message: Option<GoToView>,
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
    pub fn change_route(&mut self, router_message: GoToView) {
        self.router_message = Some(router_message);
    }
    pub fn batch(self) -> (Command<M>, Option<GoToView>) {
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

pub enum GoToView {
    SettingsContacts,
    Chat,
    Channels,
    About,
    Network,
    Settings,
    ChatTo(DbContact),
    Welcome,
    Login,
    Logout,
    Back,
}

#[derive(Debug, Clone)]
pub enum Message {
    Home(Box<home::Message>),
    Settings(Box<settings::Message>),
    Login(Box<login::Message>),
    Logout(Box<logout::Message>),
    Welcome(Box<welcome::Message>),
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
        router_message: GoToView,
        conn: &mut BackEndConnection,
    ) -> Result<Command<Message>, BackendClosed> {
        match router_message {
            GoToView::Logout => self.next_state(ViewState::Logout {
                state: logout::State::new(),
            }),
            GoToView::Back => self.back(conn),
            GoToView::SettingsContacts => self.next_state(ViewState::settings_contacts(conn)?),
            GoToView::Chat => self.next_state(ViewState::chat(conn)?),
            GoToView::Channels => self.next_state(ViewState::channels(conn)?),
            GoToView::About => self.next_state(ViewState::settings_about(conn)),
            GoToView::Network => self.next_state(ViewState::settings_network(conn)?),
            GoToView::Settings => self.next_state(ViewState::settings(conn)?),
            GoToView::ChatTo(db_contact) => {
                let state = ViewState::chat_contact(db_contact, conn)?;
                self.next_state(state);
            }
            GoToView::Login => {
                let (state, command) = ViewState::login(conn);
                self.next_state(state);
                return Ok(command);
            }
            GoToView::Welcome => {
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
            ViewState::Settings { state } => state.subscription().map(map_settings_msg),
            ViewState::Home { state } => state.subscription().map(map_home_msg),
            ViewState::Welcome { state } => state.subscription().map(map_welcome_msg),
            _ => Subscription::none(),
        }
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<'_, Self::Message> {
        match self {
            Self::Welcome { state } => state.view(selected_theme).map(map_welcome_msg),
            Self::Home { state } => state.view(selected_theme).map(map_home_msg),
            Self::Login { state } => state.view(selected_theme).map(map_login_msg),
            Self::Logout { state } => state.view(selected_theme).map(map_logout_msg),
            Self::Settings { state } => state.view(selected_theme).map(map_settings_msg),
        }
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let command = match self {
            ViewState::Logout { state } => state.backend_event(event, conn)?.map(map_logout_msg),
            ViewState::Home { state } => state.backend_event(event, conn)?.map(map_home_msg),
            ViewState::Settings { state } => {
                state.backend_event(event, conn)?.map(map_settings_msg)
            }
            ViewState::Welcome { state } => state.backend_event(event, conn)?.map(map_welcome_msg),
            ViewState::Login { state } => state.backend_event(event, conn)?.map(map_login_msg),
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
                    if let Message::Welcome(msg) = message {
                        break 'command state.update(*msg, conn)?.map(map_welcome_msg);
                    }
                }
                Self::Home { state } => {
                    if let Message::Home(msg) = message {
                        break 'command state.update(*msg, conn)?.map(map_home_msg);
                    }
                }
                Self::Login { state } => {
                    if let Message::Login(msg) = message {
                        break 'command state.update(*msg, conn)?.map(map_login_msg);
                    }
                }
                Self::Logout { state } => {
                    if let Message::Logout(msg) = message {
                        break 'command state.update(*msg, conn)?.map(map_logout_msg);
                    }
                }
                Self::Settings { state } => {
                    if let Message::Settings(msg) = message {
                        break 'command state.update(*msg, conn)?.map(map_settings_msg);
                    }
                }
            };
            RouterCommand::new()
        };

        Ok(command)
    }
}

fn map_settings_msg(message: settings::Message) -> Message {
    Message::Settings(Box::new(message))
}
fn map_login_msg(message: login::Message) -> Message {
    Message::Login(Box::new(message))
}
fn map_logout_msg(message: logout::Message) -> Message {
    Message::Logout(Box::new(message))
}
fn map_home_msg(message: home::Message) -> Message {
    Message::Home(Box::new(message))
}
fn map_welcome_msg(message: welcome::Message) -> Message {
    Message::Welcome(Box::new(message))
}
