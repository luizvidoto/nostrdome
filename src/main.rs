#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub(crate) mod components;
pub(crate) mod consts;
pub(crate) mod db;
pub(crate) mod error;
pub(crate) mod icon;
pub(crate) mod net;
pub(crate) mod style;
pub(crate) mod types;
pub(crate) mod utils;
pub(crate) mod views;
pub(crate) mod widget;

use components::text::title;
use dotenv::dotenv;
use iced::{
    executor,
    widget::{button, column, container, text},
    window, Application, Command, Length, Settings,
};
use net::{backend_connect, events::Event, BackEndConnection};
use nostr_sdk::Keys;
use tracing::{metadata::LevelFilter, Subscriber};
use tracing_subscriber::{
    fmt::SubscriberBuilder, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};
use views::{
    login::{self, BasicProfile},
    settings::{self, appearance},
    welcome, Router,
};
use widget::Element;

use crate::db::DbRelay;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    BackEndEvent(Event),
    LoginMessage(login::Message),
    WelcomeMessage(welcome::Message),
    ToLogin,
}
pub enum State {
    Login {
        state: login::State,
    },
    LoggingOut {
        keys: Keys,
        conn: BackEndConnection,
    },
    LoadingBackend {
        keys: Keys,
        new_profile: Option<BasicProfile>,
    },
    BackendLoaded {
        keys: Keys,
        conn: BackEndConnection,
    },
    Welcome {
        keys: Keys,
        conn: BackEndConnection,
        state: welcome::State,
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
    pub fn logging_out(keys: &mut Keys, conn: &mut BackEndConnection) -> Self {
        Self::LoggingOut {
            keys: keys.to_owned(),
            conn: conn.to_owned(),
        }
    }
    pub fn welcome(keys: &mut Keys, conn: &mut BackEndConnection) -> Self {
        Self::Welcome {
            state: welcome::State::new(),
            keys: keys.to_owned(),
            conn: conn.to_owned(),
        }
    }
    pub fn load_back(keys: Keys) -> Self {
        Self::LoadingBackend {
            keys: keys,
            new_profile: None,
        }
    }
    pub fn load_back_with_profile(keys: Keys, profile: BasicProfile) -> Self {
        Self::LoadingBackend {
            keys: keys,
            new_profile: Some(profile),
        }
    }
    pub fn backend_loaded(keys: &mut Keys, conn: &mut BackEndConnection) -> Self {
        Self::BackendLoaded {
            keys: keys.to_owned(),
            conn: conn.to_owned(),
        }
    }
    pub fn error(message: &str) -> Self {
        Self::OutsideError {
            message: message.into(),
        }
    }
    pub fn to_app(keys: &mut Keys, conn: &mut BackEndConnection) -> Self {
        let router = Router::new(conn);
        Self::App {
            keys: keys.to_owned(),
            conn: conn.to_owned(),
            router,
        }
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
            State::LoadingBackend { keys, .. }
            | State::Welcome { keys, .. }
            | State::BackendLoaded { keys, .. }
            | State::App { keys, .. }
            | State::LoggingOut { keys, .. } => backend_connect(keys)
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
            State::Welcome { state, .. } => state.view().map(Message::WelcomeMessage),
            State::LoadingBackend { .. } | State::BackendLoaded { .. } => {
                inform_card("Loading App", "Please wait...")
            }
            State::App { router, .. } => router.view().map(Message::RouterMessage),
            State::OutsideError { message } => {
                let error_msg = text(message);
                let back_btn = button("Login").on_press(Message::ToLogin);
                let content: Element<_> = column![error_msg, back_btn].spacing(10).into();
                inform_card("Error", content)
            }
            State::LoggingOut { .. } => inform_card("Logging out", "Please wait..."),
        };
        content.into()
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ToLogin => {
                self.state = State::login();
            }
            Message::WelcomeMessage(welcome_msg) => {
                if let State::Welcome { state, conn, .. } = &mut self.state {
                    match welcome_msg {
                        welcome::Message::ToApp => {
                            conn.send(net::Message::StoreFirstLogin);
                            conn.send(net::Message::PrepareClient);
                            state.update(welcome::Message::ToApp);
                        }
                        other => state.update(other),
                    }
                }
            }
            Message::LoginMessage(login_msg) => {
                if let State::Login { state } = &mut self.state {
                    if let Some(msg) = state.update(login_msg) {
                        match msg {
                            login::Message::CreateAccountSubmitSuccess((profile, keys)) => {
                                self.state = State::load_back_with_profile(keys, profile);
                            }
                            login::Message::LoginSuccess(keys) => {
                                self.state = State::load_back(keys);
                            }
                            _ => (),
                        }
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
                Event::None => (),
                Event::Disconnected => {
                    tracing::info!("Disconnected from backend");
                }
                Event::BackendClosed => {
                    tracing::info!("Database closed");
                    if let State::App { keys, conn, .. } = &mut self.state {
                        self.state = State::logging_out(keys, conn);
                    }
                }
                Event::LoggedOut => {
                    tracing::info!("Logged out");
                    self.state = State::login();
                }
                Event::FirstLogin => {
                    tracing::info!("First login");
                    if let State::BackendLoaded { keys, conn } = &mut self.state {
                        self.state = State::welcome(keys, conn);
                    }
                }
                Event::Connected(mut ready_conn) => {
                    tracing::info!("Connected to backend");
                    if let State::LoadingBackend { keys, new_profile } = &mut self.state {
                        if let Some(profile) = new_profile {
                            ready_conn.send(net::Message::CreateAccount(profile.to_owned()));
                        }
                        ready_conn.send(net::Message::FetchLatestVersion);
                        ready_conn.send(net::Message::QueryFirstLogin);
                        self.state = State::backend_loaded(keys, &mut ready_conn);
                    }
                }
                Event::Error(e) => {
                    tracing::error!("{}", e);
                }
                Event::FinishedPreparing => {
                    tracing::info!("Finished preparing client");
                    match &mut self.state {
                        State::Welcome { keys, conn, .. } | State::BackendLoaded { conn, keys } => {
                            // only for testing
                            if let Ok(db_relay) = DbRelay::from_str("ws://192.168.85.151:8080") {
                                conn.send(net::Message::AddRelay(db_relay));
                            }
                            if let Ok(db_relay) = DbRelay::from_str("ws://192.168.15.151:8080") {
                                conn.send(net::Message::AddRelay(db_relay));
                            }
                            self.state = State::to_app(keys, conn);
                        }
                        _ => (),
                    }
                }
                Event::RelayCreated(db_relay) => {
                    tracing::info!("Relay created: {:?}", db_relay);
                    if let State::App { router, conn, .. } = &mut self.state {
                        conn.send(net::Message::ConnectToRelay(db_relay.clone()));
                        return router
                            .backend_event(Event::RelayCreated(db_relay), conn)
                            .map(Message::RouterMessage);
                    }
                }
                other => {
                    tracing::info!("Backend event: {:?}", other);
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

fn inform_card<'a>(
    title_text: &str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = column![title(title_text).width(Length::Shrink), content.into()].spacing(10);
    let inner = container(content)
        .padding(30)
        .style(style::Container::ContactList);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(style::Container::ChatContainer)
        .into()
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Cria um filtro de ambiente que define o nível de log padrão para todas as bibliotecas como ERROR e o nível de log do seu aplicativo como INFO
    let filter = EnvFilter::from_default_env()
        .add_directive("nostrdome=info".parse().unwrap())
        .add_directive("warn".parse().unwrap());

    let subscriber = SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::default()) // Adicione esta linha para incluir eventos de spans
        .finish()
        .with(tracing_error::ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    tracing::info!("Starting up");

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
