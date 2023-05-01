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
use net::{backend_connect, database, nostr_client, BackEndConnection, Connection};
use nostr_sdk::{prelude::FromSkStr, Keys};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use views::{
    login,
    settings::{self, appearance},
    Router,
};
use widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    BackEndEvent(net::Event),
    LoginMessage(login::Message),
    ToLogin,
    DatabaseStarted,
    NostrClientStarted,
}
#[derive(Debug)]
pub enum State {
    Login {
        state: login::State,
    },
    App {
        keys: Keys,
        db_conn: BackEndConnection<database::Message>,
        ns_conn: BackEndConnection<nostr_client::Message>,
        router: Router,
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
    pub fn to_app(secret_key: &str) -> Self {
        match Keys::from_sk_str(secret_key) {
            Ok(keys) => Self::app(keys),
            Err(e) => Self::OutsideError {
                message: e.to_string(),
            },
        }
    }
    pub fn app(keys: Keys) -> Self {
        let mut db_conn = BackEndConnection::new();
        let router = Router::new(&mut db_conn);
        Self::App {
            keys,
            db_conn,
            ns_conn: BackEndConnection::new(),
            router,
        }
    }
}

#[derive(Debug)]
pub struct App {
    state: State,
    color_theme: Option<style::Theme>,
}
pub enum NotificationState {
    Starting,
    Running,
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
            State::App {
                keys,
                db_conn,
                ns_conn,
                ..
            } => backend_connect(keys, db_conn, ns_conn)
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
                        self.state = State::to_app(&secret_key);
                    } else {
                        state.update(login_msg);
                    }
                }
            }
            Message::RouterMessage(msg) => {
                if let State::App {
                    ns_conn,
                    db_conn,
                    router,
                    ..
                } = &mut self.state
                {
                    if let views::Message::SettingsMsg(settings_msg) = msg.clone() {
                        if let settings::Message::AppearanceMessage(appearance_msg) = settings_msg {
                            if let appearance::Message::ChangeTheme(theme) = appearance_msg {
                                self.color_theme = Some(theme);
                            }
                        }
                    }

                    return router
                        .update(msg, db_conn, ns_conn, self.color_theme)
                        .map(Message::RouterMessage);
                }
            }
            Message::BackEndEvent(event) => match event {
                net::Event::None => (),
                net::Event::DbEvent(db_event) => match db_event {
                    database::Event::IsProcessing => {
                        // tracing::warn!("Database is processing messages");
                    }
                    database::Event::FinishedProcessing => {
                        // tracing::warn!("Database finished");
                        if let State::App { db_conn, .. } = &mut self.state {
                            db_conn.send(database::Message::ProcessMessages);
                        }
                    }
                    database::Event::GotClientConfig(config) => {
                        tracing::warn!("Got config for client");
                        if let State::App { ns_conn, .. } = &mut self.state {
                            ns_conn.send(nostr_client::Message::PrepareClient(config));
                        }
                    }
                    database::Event::DbConnected => {
                        tracing::warn!("Received Database Connected Event");
                        // send relays from db to the client?
                        // if let State::App { db_conn, .. } = &mut self.state {
                        //     db_conn.send(database::Message::ProcessMessages);
                        // }
                    }
                    database::Event::DbDisconnected => {
                        // self.state = State::login();
                        tracing::warn!("Database Disconnected");
                    }
                    other => {
                        if let State::App {
                            ns_conn,
                            db_conn,
                            router,
                            ..
                        } = &mut self.state
                        {
                            return router
                                .backend_event(net::Event::DbEvent(other), db_conn, ns_conn)
                                .map(Message::RouterMessage);
                        }
                    }
                },
                net::Event::NostrClientEvent(ns_event) => match ns_event {
                    nostr_client::Event::ReceivedEvent((url, event)) => {
                        // tracing::warn!("Received Nostr Event: {:?}", event);
                        if let State::App { db_conn, .. } = &mut self.state {
                            db_conn.send(database::Message::ReceivedEvent((url, event)));
                        }
                    }
                    nostr_client::Event::ReceivedRelayMessage((url, msg)) => {
                        // tracing::warn!("Received Nostr Event: {:?}", event);
                        if let State::App { db_conn, .. } = &mut self.state {
                            db_conn.send(database::Message::ReceivedRelayMessage((url, msg)));
                        }
                    }

                    nostr_client::Event::IsProcessing => {
                        // tracing::warn!("Database is processing messages");
                    }
                    nostr_client::Event::FinishedProcessing => {
                        // tracing::warn!("Database finished");
                        if let State::App { ns_conn, .. } = &mut self.state {
                            ns_conn.send(nostr_client::Message::ProcessMessages);
                        }
                    }
                    nostr_client::Event::FinishedPreparing => {
                        tracing::warn!("Finished preparing client");
                    }
                    nostr_client::Event::NostrConnected => {
                        tracing::warn!("Received Nostr Client Connected Event");
                        if let State::App { db_conn, .. } = &mut self.state {
                            db_conn.send(database::Message::PrepareClient);
                        }
                    }
                    nostr_client::Event::NostrDisconnected => {
                        tracing::warn!("Nostr Client Disconnected");
                    }
                    other => {
                        if let State::App {
                            ns_conn,
                            db_conn,
                            router,
                            ..
                        } = &mut self.state
                        {
                            return router
                                .backend_event(
                                    net::Event::NostrClientEvent(other),
                                    db_conn,
                                    ns_conn,
                                )
                                .map(Message::RouterMessage);
                        }
                    }
                },

                net::Event::Error(e) => {
                    tracing::error!("{}", e);
                }
            },
        };

        Command::none()
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
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
