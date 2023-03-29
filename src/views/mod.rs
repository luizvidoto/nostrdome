use iced::Element;

use crate::net::Connection;

mod chat;
mod settings;

#[derive(Debug, Clone)]
pub enum Message {
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
}
#[derive(Debug)]
pub struct Router {
    prev_state: Option<State>,
    state: State,
}
impl Router {
    fn next_state(&mut self, next: State) {
        let old_state = std::mem::replace(&mut self.state, next);
        self.prev_state = Some(old_state);
    }
    fn next_state_skip(&mut self, next: State) {
        self.state = next;
    }
    fn back(&mut self) {
        if let Some(s) = self.prev_state.take() {
            self.state = s;
        }
    }
    pub fn view(&self) -> Element<Message> {
        self.state.view()
    }
    pub fn update(&mut self, message: Message, conn: &mut Connection) {
        match message {
            Message::ChatMsg(msg) => {
                if let State::Chat { state } = &mut self.state {
                    match msg {
                        chat::Message::NavSettingsPress => self.next_state(State::settings()),
                        _ => state.update(msg.clone(), conn),
                    }
                }
            }
            Message::SettingsMsg(msg) => {
                if let State::Settings { state } = &mut self.state {
                    match msg {
                        settings::Message::NavEscPress => self.back(),
                        _ => state.update(msg.clone()),
                    }
                }
            }
        }
    }
    pub fn default() -> Self {
        Self {
            prev_state: None,
            state: State::chat(),
        }
    }
}

#[derive(Debug)]
pub enum State {
    Chat { state: chat::State },
    Settings { state: settings::State },
}
impl State {
    pub fn chat() -> Self {
        Self::Chat {
            state: chat::State::new(),
        }
    }
    pub fn settings() -> Self {
        Self::Settings {
            state: settings::State::default(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
        }
    }
}
