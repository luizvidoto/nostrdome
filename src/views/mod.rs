use iced::widget::text;
use iced::Element;
use nostr_sdk::Keys;

use crate::net::{self, database::DbConnection, nostr::NostrConnection};

mod chat;
pub mod login;
mod settings;

#[derive(Debug, Clone)]
pub enum Message {
    ChangeView,
    ChatMsg(chat::Message),
    SettingsMsg(settings::Message),
}
#[derive(Debug)]
pub struct Router {
    previous_state: Option<ViewState>,
    state: ViewState,
}
impl Router {
    pub fn new(keys: &Keys, db_conn: &mut DbConnection) -> Self {
        Self {
            previous_state: None,
            state: ViewState::chat(keys, db_conn),
        }
    }
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
    pub fn db_event(
        &mut self,
        event: net::database::Event,
        db_conn: &mut DbConnection,
        nostr_conn: &mut NostrConnection,
    ) {
        match &mut self.state {
            ViewState::Loading => (),
            ViewState::Chat { state } => state.db_event(event, db_conn, nostr_conn),
            ViewState::Settings { state } => state.db_event(event, db_conn, nostr_conn),
        }
    }
    pub fn nostr_event(
        &mut self,
        event: net::nostr::Event,
        keys: &Keys,
        db_conn: &mut DbConnection,
        nostr_conn: &mut NostrConnection,
    ) {
        match event {
            net::nostr::Event::RelaysConnected => {
                if let Err(e) = nostr_conn.send(net::nostr::Message::ListOwnEvents) {
                    tracing::error!("{}", e);
                };
            }
            net::nostr::Event::GotOwnEvents(events) => {
                tracing::info!("Got events: {}", events.len());
                for event in events {
                    if let Err(e) = db_conn.send(net::database::Message::InsertEvent {
                        keys: keys.clone(),
                        event,
                    }) {
                        tracing::error!("{}", e);
                    }
                }
            }
            net::nostr::Event::NostrEvent(event) => {
                tracing::info!("Nostr Event: {:?}", event);
                if let Err(e) = db_conn.send(net::database::Message::InsertEvent {
                    keys: keys.clone(),
                    event,
                }) {
                    tracing::error!("{}", e);
                }
            }
            event => {
                println!("Other: {:?}", event);
            }
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        db_conn: &mut DbConnection,
        nostr_conn: &mut NostrConnection,
    ) {
        match message {
            Message::ChangeView => (),
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
}

#[derive(Debug)]
pub enum ViewState {
    Chat { state: chat::State },
    Settings { state: settings::State },
    Loading,
}

impl ViewState {
    pub fn loading() -> Self {
        Self::Loading
    }
    pub fn chat(keys: &Keys, db_conn: &mut DbConnection) -> Self {
        Self::Chat {
            state: chat::State::new(keys, db_conn),
        }
    }
    pub fn settings(db_conn: &mut DbConnection) -> Self {
        Self::Settings {
            state: settings::State::new(db_conn),
        }
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Loading => text("Loading").into(),
            Self::Chat { state } => state.view().map(Message::ChatMsg),
            Self::Settings { state } => state.view().map(Message::SettingsMsg),
        }
    }
}
