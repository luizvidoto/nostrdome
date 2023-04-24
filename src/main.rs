pub mod components;
pub mod crypto;
pub mod db;
pub mod error;
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
use net::{backend_connect, BackEndConnection};
use nostr_sdk::{prelude::FromSkStr, Keys};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use views::{login, Router};
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
        back_conn: BackEndConnection,
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
    pub fn loaded(keys: Keys, mut back_conn: BackEndConnection) -> Self {
        let router = Router::new(&mut back_conn);
        Self::Loaded {
            keys,
            back_conn,
            router,
        }
    }
}

#[derive(Debug)]
pub struct App {
    state: State,
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
            },
            Command::none(),
        )
    }
    fn title(&self) -> String {
        String::from("NostrDome")
    }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let backend_subscription = match &self.state {
            State::Login { .. } | State::OutsideError { .. } => iced::Subscription::none(),
            State::Loading { keys, .. } | State::Loaded { keys, .. } => {
                backend_connect(keys).map(Message::BackEndEvent)
            }
        };
        let app_sub = match &self.state {
            State::Loaded { router, .. } => router.subscription().map(Message::RouterMessage),
            _ => iced::Subscription::none(),
        };
        iced::Subscription::batch(vec![backend_subscription, app_sub])
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
        match message {
            Message::ToLogin => self.state = State::login(),
            Message::LoginMessage(login_msg) => {
                if let State::Login { state } = &mut self.state {
                    if let login::Message::SubmitPress(secret_key) = login_msg {
                        self.state = State::loading(&secret_key);
                    } else {
                        state.update(login_msg);
                    }
                }
            }
            Message::RouterMessage(msg) => {
                if let State::Loaded {
                    back_conn, router, ..
                } = &mut self.state
                {
                    return router.update(msg, back_conn).map(Message::RouterMessage);
                }
            }
            Message::BackEndEvent(event) => match event {
                net::Event::Connected(mut back_conn) => {
                    tracing::info!("Received BackEnd Connected Event");
                    if let State::Loading { keys } = &mut self.state {
                        back_conn.send(net::Message::ConnectRelays);
                        self.state = State::loaded(keys.clone(), back_conn);
                    }
                }
                net::Event::Disconnected => {
                    self.state = State::login();
                }
                net::Event::Error(e) => {
                    tracing::error!("{}", e);
                }
                ev => {
                    if let State::Loaded {
                        router, back_conn, ..
                    } = &mut self.state
                    {
                        return router
                            .back_end_event(ev, back_conn)
                            .map(Message::RouterMessage);
                    }
                }
            },
        }

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

    tracing::info!("Starting up");

    App::run(Settings {
        id: Some(String::from("nostrdome")),
        window: window::Settings {
            size: (600, 800),
            min_size: Some((600, 400)),
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        ..Default::default()
    })
    .expect("Failed to run app");
}
