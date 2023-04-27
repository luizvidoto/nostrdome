use iced::{Command, Subscription};

use crate::{
    net::{self, BackEndConnection},
    style,
    widget::Element,
};

mod channels;
mod chat;
pub mod login;
pub mod settings;

#[derive(Debug, Clone)]
pub enum Message {
    ChannelsMsg(channels::Message),
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
}
#[derive(Debug)]
pub struct Router {
    previous_state: Option<ViewState>,
    state: ViewState,
}
impl Router {
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        Self {
            previous_state: None,
            state: ViewState::chat(back_conn),
        }
    }
    fn next_state(&mut self, next: ViewState) {
        let old_state = std::mem::replace(&mut self.state, next);
        self.previous_state = Some(old_state);
    }
    fn _next_state_skip(&mut self, next: ViewState) {
        self.state = next;
    }
    fn _back(&mut self) {
        if let Some(s) = self.previous_state.take() {
            self.state = s;
        }
    }
    pub fn subscription(&self) -> Subscription<Message> {
        self.state.subscription()
    }
    pub fn view(&self) -> Element<Message> {
        self.state.view()
    }
    pub fn back_end_event(
        &mut self,
        event: net::Event,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match event {
            event => match &mut self.state {
                ViewState::Channels { state } => state
                    .backend_event(event, back_conn)
                    .map(Message::ChannelsMsg),
                ViewState::Chat { state } => {
                    state.backend_event(event, back_conn).map(Message::ChatMsg)
                }
                ViewState::Settings { state } => state
                    .back_end_event(event, back_conn)
                    .map(Message::SettingsMsg),
            },
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> Command<Message> {
        match message {
            Message::ChannelsMsg(msg) => {
                if let ViewState::Channels { state } = &mut self.state {
                    match msg {
                        channels::Message::GoToChat => self.next_state(ViewState::chat(back_conn)),
                        msg => state.update(msg, back_conn),
                    }
                }
            }
            Message::ChatMsg(msg) => {
                if let ViewState::Chat { state } = &mut self.state {
                    match msg {
                        chat::Message::AddContactPress => {
                            self.next_state(ViewState::settings_contacts(back_conn))
                        }
                        chat::Message::GoToChannelsPress => {
                            self.next_state(ViewState::channels(back_conn))
                        }
                        chat::Message::NavSettingsPress => {
                            self.next_state(ViewState::settings(back_conn))
                        }
                        msg => state.update(msg, back_conn),
                    }
                }
            }
            Message::SettingsMsg(msg) => {
                if let ViewState::Settings { state } = &mut self.state {
                    match msg {
                        settings::Message::NavEscPress => {
                            self.next_state(ViewState::chat(back_conn))
                        }
                        msg => {
                            return state
                                .update(msg, back_conn, selected_theme)
                                .map(Message::SettingsMsg);
                        }
                    }
                }
            }
        };
        Command::none()
    }
}

#[derive(Debug)]
pub enum ViewState {
    Channels { state: channels::State },
    Chat { state: chat::State },
    Settings { state: settings::Settings },
}

impl ViewState {
    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            ViewState::Settings { state } => state.subscription().map(Message::SettingsMsg),
            _ => Subscription::none(),
        }
    }
    pub fn channels(_back_conn: &mut BackEndConnection) -> Self {
        Self::Channels {
            state: channels::State::new(),
        }
    }
    pub fn chat(back_conn: &mut BackEndConnection) -> Self {
        Self::Chat {
            state: chat::State::new(back_conn),
        }
    }
    pub fn settings(back_conn: &mut BackEndConnection) -> Self {
        Self::Settings {
            state: settings::Settings::new(back_conn),
        }
    }
    pub fn settings_contacts(back_conn: &mut BackEndConnection) -> Self {
        Self::Settings {
            state: settings::Settings::contacts(back_conn),
        }
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Channels { state } => state.view().map(Message::ChannelsMsg),
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
        }
    }
}
