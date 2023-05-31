use iced::{Command, Subscription};

use crate::{
    db::DbContact,
    net::{BackEndConnection, BackendEvent},
    style,
    widget::Element,
};

mod channels;
mod chat;
pub(crate) mod login;
pub(crate) mod modal;
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
}

#[derive(Debug, Clone)]
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
    ChannelsMsg(channels::Message),
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
    LoginMsg(login::Message),
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
    pub fn view(&self) -> Element<Message> {
        self.state.view()
    }

    pub fn change_route(
        &mut self,
        router_message: RouterMessage,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match router_message {
            RouterMessage::GoToLogout => self.next_state(ViewState::Logout {
                state: logout::State::new(),
            }),
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
                let state = ViewState::chat_contact(db_contact, conn);
                self.next_state(state);
            }
            RouterMessage::GoToLogin => {
                let (state, command) = ViewState::login(conn);
                self.next_state(state);
                return command;
            }
            RouterMessage::GoToWelcome => {
                let (state, command) = ViewState::welcome(conn);
                self.next_state(state);
                return command;
            }
        }
        Command::none()
    }

    pub fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        let (command, router_message) = self.state.backend_event(event, conn);
        if let Some(router_message) = router_message {
            let change_cmd = self.change_route(router_message, conn);
            Command::batch(vec![command, change_cmd])
        } else {
            command
        }
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
    Logout { state: logout::State },
    Channels { state: channels::State },
    Chat { state: chat::State },
    Settings { state: settings::Settings },
    Login { state: login::State },
    Welcome { state: welcome::State },
}

impl ViewState {
    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            ViewState::Settings { state } => state.subscription().map(Message::SettingsMsg),
            ViewState::Chat { state } => state.subscription().map(Message::ChatMsg),
            ViewState::Welcome { state } => state.subscription().map(Message::WelcomeMsg),
            _ => Subscription::none(),
        }
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Logout { state } => state.view(),
            Self::Channels { state } => state.view().map(Message::ChannelsMsg),
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
            Self::Login { state } => state.view().map(Message::LoginMsg),
            Self::Welcome { state } => state.view().map(Message::WelcomeMsg),
        }
    }
    pub fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> (Command<Message>, Option<RouterMessage>) {
        let mut commands = vec![];
        let mut router_message = None;

        match self {
            ViewState::Logout { state } => {
                let (cmd, msg) = state.backend_event(event, conn);
                router_message = msg;
                commands.push(cmd);
            }
            ViewState::Channels { state } => {
                let (cmd, msg) = state.backend_event(event, conn);
                router_message = msg;
                commands.push(cmd.map(Message::ChannelsMsg));
            }
            ViewState::Chat { state } => {
                let (cmds, msg) = state.backend_event(event, conn).batch();
                router_message = msg;
                commands.push(cmds.map(Message::ChatMsg));
            }
            ViewState::Settings { state } => {
                let (cmds, msg) = state.backend_event(event, conn).batch();
                router_message = msg;
                commands.push(cmds.map(Message::SettingsMsg));
            }
            ViewState::Welcome { state } => {
                let (cmd, msg) = state.backend_event(event, conn);
                router_message = msg;
                commands.push(cmd.map(Message::WelcomeMsg));
            }
            ViewState::Login { state } => {
                let (cmd, msg) = state.backend_event(event, conn);
                router_message = msg;
                commands.push(cmd.map(Message::LoginMsg));
            }
        }

        (Command::batch(commands), router_message)
    }
    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> (Command<Message>, Option<RouterMessage>) {
        let mut commands = vec![];
        let mut router_message = None;
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
            Message::ChatMsg(chat_msg) => {
                if let ViewState::Chat { state } = self {
                    let (cmds, msg) = state.update(chat_msg, conn).batch();
                    router_message = msg;
                    commands.push(cmds.map(Message::ChatMsg));
                }
            }
            Message::SettingsMsg(settings_msg) => {
                if let ViewState::Settings { state } = self {
                    let (cmds, msg) = state.update(settings_msg, conn, selected_theme).batch();
                    router_message = msg;
                    commands.push(cmds.map(Message::SettingsMsg));
                }
            }
            Message::WelcomeMsg(welcome_msg) => {
                if let ViewState::Welcome { state } = self {
                    let (cmd, msg) = state.update(welcome_msg, conn);
                    commands.push(cmd.map(Message::WelcomeMsg));
                    router_message = msg;
                }
            }
            Message::LoginMsg(login_msg) => {
                if let ViewState::Login { state } = self {
                    let (cmd, msg) = state.update(login_msg, conn);
                    commands.push(cmd.map(Message::LoginMsg));
                    router_message = msg;
                }
            }
        };

        (Command::batch(commands), router_message)
    }

    fn login(_conn: &mut BackEndConnection) -> (ViewState, Command<Message>) {
        let state = login::State::new();
        (Self::Login { state }, Command::none())
    }

    fn welcome(_conn: &mut BackEndConnection) -> (ViewState, Command<Message>) {
        let state = welcome::State::new();
        (Self::Welcome { state }, Command::none())
    }

    fn chat_contact(db_contact: DbContact, conn: &mut BackEndConnection) -> ViewState {
        let state = chat::State::chat_to(db_contact, conn);
        Self::Chat { state }
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

mod logout {
    use crate::components::inform_card;
    use crate::net::{BackEndConnection, BackendEvent};
    use crate::widget::Element;
    use iced::Command;

    use super::RouterMessage;

    #[derive(Debug, Clone)]
    pub struct State {}
    impl State {
        pub fn new() -> Self {
            Self {}
        }

        pub fn backend_event<M>(
            &mut self,
            event: BackendEvent,
            _conn: &mut BackEndConnection,
        ) -> (Command<M>, Option<RouterMessage>) {
            let command = Command::none();
            let mut router_message = None;
            match event {
                BackendEvent::LogoutSuccess => {
                    router_message = Some(RouterMessage::GoToLogin);
                }
                _ => (),
            }
            (command, router_message)
        }

        pub fn view<'a, M: 'a>(&'a self) -> Element<M> {
            inform_card("Logging out", "Please wait...").into()
        }
    }
}
