use iced::Element;

use crate::net::{self, database::DbConnection, nostr::NostrConnection};

mod chat;
pub mod login;
mod settings;

#[derive(Debug, Clone)]
pub enum Message {
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
}
#[derive(Debug)]
pub struct Router {
    previous_state: Option<ViewState>,
    state: ViewState,
}
impl Router {
    fn next_state(&mut self, next: ViewState) {
        let old_state = std::mem::replace(&mut self.state, next);
        self.previous_state = Some(old_state);
    }
    fn _next_state_skip(&mut self, next: ViewState) {
        self.state = next;
    }
    fn back(&mut self) {
        if let Some(s) = self.previous_state.take() {
            self.state = s;
        }
    }
    pub fn view(&self) -> Element<Message> {
        self.state.view()
    }
    pub fn db_event(&mut self, event: net::database::Event, db_conn: &mut DbConnection) {
        match &mut self.state {
            ViewState::Chat { state } => state.db_event(event, db_conn),
            ViewState::Settings { state } => state.db_event(event, db_conn),
        }
    }
    pub fn nostr_event(
        &mut self,
        event: net::nostr::Event,
        _db_conn: &mut DbConnection,
        _nostr_conn: &mut NostrConnection,
    ) {
        println!("{:?}", event);
        // match &mut self.state {
        //     ViewState::Chat { state } => state.db_event(event, db_conn),
        //     ViewState::Settings { state } => state.db_event(event, db_conn),
        // }
    }
    pub fn update(
        &mut self,
        message: Message,
        db_conn: &mut DbConnection,
        nostr_conn: &mut NostrConnection,
    ) {
        match message {
            Message::ChatMsg(msg) => {
                if let ViewState::Chat { state } = &mut self.state {
                    match msg {
                        chat::Message::NavSettingsPress => {
                            self.next_state(ViewState::settings(db_conn))
                        }
                        _ => state.update(msg.clone(), db_conn, nostr_conn),
                    }
                }
            }
            Message::SettingsMsg(msg) => {
                if let ViewState::Settings { state } = &mut self.state {
                    match msg {
                        settings::Message::NavEscPress => self.back(),
                        _ => state.update(msg.clone(), db_conn),
                    }
                }
            }
        };
    }
    pub fn default() -> Self {
        Self {
            previous_state: None,
            state: ViewState::chat(),
        }
    }
}

#[derive(Debug)]
pub enum ViewState {
    Chat { state: chat::State },
    Settings { state: settings::State },
}

impl ViewState {
    pub fn chat() -> Self {
        Self::Chat {
            state: chat::State::new(),
        }
    }
    pub fn settings(db_conn: &mut DbConnection) -> Self {
        Self::Settings {
            state: settings::State::new(db_conn),
        }
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
        }
    }
}
