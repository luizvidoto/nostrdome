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
use net::{backend_connect, DBConnection, NostrConnection};
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
}
#[derive(Debug)]
pub enum State {
    Login {
        state: login::State,
    },
    Loading {
        keys: Keys,
    },
    Loaded {
        keys: Keys,
        db_conn: DBConnection,
        ns_conn: Option<NostrConnection>,
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
    pub fn loading(secret_key: &str) -> Self {
        match Keys::from_sk_str(secret_key) {
            Ok(keys) => Self::Loading { keys },
            Err(e) => Self::OutsideError {
                message: e.to_string(),
            },
        }
    }
    pub fn loaded(keys: Keys, mut db_conn: DBConnection) -> Self {
        let router = Router::new(&mut db_conn);
        Self::Loaded {
            keys,
            db_conn,
            ns_conn: None,
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
            State::Loading { keys, .. } | State::Loaded { keys, .. } => backend_connect(keys)
                .iter()
                .map(|stream| stream.map(Message::BackEndEvent))
                .collect(),
        };
        let app_sub = match &self.state {
            State::Loaded { router, .. } => router.subscription().map(Message::RouterMessage),
            _ => iced::Subscription::none(),
        };
        let mut subscriptions = backend_subscription;
        subscriptions.push(app_sub);
        iced::Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<Self::Message> {
        let content: Element<_> = match &self.state {
            State::Login { state } => state.view().map(Message::LoginMessage),
            State::Loading { .. } => text("Loading app...").into(),
            State::Loaded { router, .. } => router.view().map(Message::RouterMessage),
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
        let command = match message {
            Message::ToLogin => {
                self.state = State::login();
                Command::none()
            }
            Message::LoginMessage(login_msg) => {
                if let State::Login { state } = &mut self.state {
                    if let login::Message::SubmitPress(secret_key) = login_msg {
                        self.state = State::loading(&secret_key);
                    } else {
                        state.update(login_msg);
                    }
                }
                Command::none()
            }
            Message::RouterMessage(msg) => {
                if let State::Loaded {
                    db_conn: db_conn,
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

                    router
                        .update(msg, db_conn, self.color_theme)
                        .map(Message::RouterMessage)
                } else {
                    Command::none()
                }
            }
            Message::BackEndEvent(event) => match event {
                net::Event::NostrConnected(mut client) => {
                    tracing::warn!("Received Nostr Client Connected Event");
                    if let State::Loaded { keys, ns_conn, .. } = &mut self.state {
                        // db_conn.send(net::Message::ConnectRelays);
                        *ns_conn = Some(client);
                    }
                    Command::none()
                }
                net::Event::DbConnected(mut db_conn) => {
                    tracing::warn!("Received Database Connected Event");
                    if let State::Loading { keys } = &mut self.state {
                        // db_conn.send(net::Message::ConnectRelays);
                        self.state = State::loaded(keys.clone(), db_conn);
                    }
                    Command::none()
                }
                net::Event::Disconnected => {
                    self.state = State::login();
                    Command::none()
                }
                net::Event::Error(e) => {
                    tracing::error!("{}", e);
                    Command::none()
                }
                ev => {
                    if let State::Loaded {
                        router, db_conn, ..
                    } = &mut self.state
                    {
                        router.db_event(ev, db_conn).map(Message::RouterMessage)
                    } else {
                        Command::none()
                    }
                }
            },
        };

        command
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
