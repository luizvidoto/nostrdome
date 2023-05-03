#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod components;
pub mod crypto;
pub mod db;
pub mod error;
pub mod icon;
pub mod net;
pub mod style;
pub mod types;
pub mod utils;
pub mod views;
pub mod widget;

use components::text::title;
use iced::{
    executor,
    widget::{button, column, container, text},
    window, Application, Command, Length, Settings,
};
use net::{backend_connect, events, BackEndConnection};
use nostr_sdk::{prelude::FromSkStr, Keys};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use views::{
    login,
    settings::{self, appearance},
    Router,
};
use widget::Element;

use crate::db::DbRelay;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    BackEndEvent(events::Event),
    LoginMessage(login::Message),
    ToLogin,
    DatabaseStarted,
    NostrClientStarted,
}
pub enum State {
    Login {
        state: login::State,
    },
    LoadingBackend {
        keys: Keys,
    },
    App {
        keys: Keys,
        router: Router,
        conn: BackEndConnection,
    },
    OutsideError {
        message: String,
    },
}
impl State {
    pub fn login() -> Self {
        Self::Login {
            state: login::State::new(),
        }
    }
    pub fn to_loading_backend(secret_key: &str) -> Self {
        match Keys::from_sk_str(secret_key) {
            Ok(keys) => Self::LoadingBackend { keys },
            Err(e) => Self::OutsideError {
                message: e.to_string(),
            },
        }
    }
    pub fn to_app(keys: Keys, mut conn: BackEndConnection) -> Self {
        let router = Router::new(&mut conn);
        Self::App { keys, conn, router }
    }
}

pub struct App {
    state: State,
    color_theme: Option<style::Theme>,
}

impl Application for App {
    type Theme = crate::style::Theme;
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                state: State::login(),
                color_theme: Some(style::Theme::default()),
            },
            Command::none(),
        )
    }
    fn theme(&self) -> Self::Theme {
        self.color_theme.unwrap()
    }
    fn title(&self) -> String {
        String::from("NostrDome")
    }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let backend_subscription: Vec<_> = match &self.state {
            State::Login { .. } | State::OutsideError { .. } => vec![],
            State::LoadingBackend { keys } | State::App { keys, .. } => backend_connect(keys)
                .into_iter()
                .map(|stream| stream.map(Message::BackEndEvent))
                .collect(),
        };
        let app_sub = match &self.state {
            State::App { router, .. } => router.subscription().map(Message::RouterMessage),
            _ => iced::Subscription::none(),
        };
        let mut subscriptions = backend_subscription;
        subscriptions.push(app_sub);
        iced::Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<Self::Message> {
        let content: Element<_> = match &self.state {
            State::Login { state } => state.view().map(Message::LoginMessage),
            State::LoadingBackend { .. } => column![title("Loading App"), text("Please wait...")]
                .spacing(10)
                .into(),
            State::App { router, .. } => router.view().map(Message::RouterMessage),
            State::OutsideError { message } => {
                let error_msg = text(message);
                let back_btn = button("Login").on_press(Message::ToLogin);
                column![title("Error"), error_msg, back_btn]
                    .spacing(10)
                    .into()
            }
        };
        container(content)
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::DatabaseStarted => {
                println!("--- DATABASE STARTED PROCESSING MESSAGES ---");
            }
            Message::NostrClientStarted => {
                println!("--- NOSTR CLIENT STARTED PROCESSING MESSAGES ---");
            }
            Message::ToLogin => {
                self.state = State::login();
            }
            Message::LoginMessage(login_msg) => {
                if let State::Login { state } = &mut self.state {
                    if let login::Message::SubmitPress(secret_key) = login_msg {
                        self.state = State::to_loading_backend(&secret_key);
                    } else {
                        state.update(login_msg);
                    }
                }
            }
            Message::RouterMessage(msg) => {
                if let State::App { router, conn, .. } = &mut self.state {
                    if let views::Message::SettingsMsg(settings_msg) = msg.clone() {
                        if let settings::Message::AppearanceMessage(appearance_msg) = settings_msg {
                            if let appearance::Message::ChangeTheme(theme) = appearance_msg {
                                self.color_theme = Some(theme);
                            }
                        }
                    }

                    return router
                        .update(msg, conn, self.color_theme)
                        .map(Message::RouterMessage);
                }
            }
            Message::BackEndEvent(event) => match event {
                events::Event::None => (),
                events::Event::Connected(mut ready_conn) => {
                    tracing::warn!("Connected to backend");
                    if let State::LoadingBackend { keys } = &mut self.state {
                        ready_conn.send(net::Message::PrepareClient);
                        self.state = State::to_app(keys.to_owned(), ready_conn);
                    }
                }
                events::Event::Error(e) => {
                    tracing::error!("{}", e);
                }
                events::Event::FinishedPreparing => {
                    tracing::warn!("Finished preparing client");
                    if let State::App { conn, .. } = &mut self.state {
                        if let Ok(db_relay) = DbRelay::from_str("ws://192.168.85.151:8080") {
                            conn.send(net::Message::AddRelay(db_relay));
                        }
                    }
                }
                events::Event::RelayCreated(db_relay) => {
                    if let State::App { router, conn, .. } = &mut self.state {
                        conn.send(net::Message::ConnectToRelay(db_relay.clone()));
                        return router
                            .backend_event(events::Event::RelayCreated(db_relay), conn)
                            .map(Message::RouterMessage);
                    }
                }
                other => {
                    if let State::App { router, conn, .. } = &mut self.state {
                        return router
                            .backend_event(other, conn)
                            .map(Message::RouterMessage);
                    }
                }
            },
        };

        Command::none()
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }

    let env_filter = EnvFilter::from_default_env();
    let max_level = match env_filter.max_level_hint() {
        Some(l) => l,
        None => LevelFilter::ERROR,
    };
    let show_debug = cfg!(debug_assertions) || max_level <= LevelFilter::DEBUG;
    tracing_subscriber::fmt::fmt()
        .with_target(false)
        .with_file(show_debug)
        .with_line_number(show_debug)
        .with_env_filter(env_filter)
        .init();

    tracing::warn!("Starting up");

    App::run(Settings {
        id: Some(String::from("nostrdome")),
        window: window::Settings {
            size: (APP_WIDTH, APP_HEIGHT),
            min_size: Some((APP_MIN_WIDTH, APP_MIN_HEIGHT)),
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        ..Default::default()
    })
    .expect("Failed to run app");
}

const APP_WIDTH: u32 = 800;
const APP_HEIGHT: u32 = 600;
const APP_MIN_WIDTH: u32 = 600;
const APP_MIN_HEIGHT: u32 = 400;
