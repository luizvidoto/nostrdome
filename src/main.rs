// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
pub(crate) use crate::error::Error;
mod config;

use components::inform_card;
use dotenv::dotenv;
use iced::{executor, subscription, window, Application, Command, Settings};
use net::{backend_connect, BackEndConnection, BackendEvent};
use tracing_subscriber::{
    fmt::SubscriberBuilder, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};
use views::Router;
use widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    BackEndEvent(BackendEvent),
    RuntimeEvent(iced::Event),
}
pub enum AppState {
    Loading,
    Loaded {
        already_sent_shutdown: bool,
        conn: BackEndConnection,
        router: Router,
    },
}
impl AppState {
    fn loaded(conn: BackEndConnection, router: Router) -> AppState {
        Self::Loaded {
            already_sent_shutdown: false,
            conn,
            router,
        }
    }
}

pub struct App {
    state: AppState,
    color_theme: Option<style::Theme>,
}

impl Application for App {
    type Theme = crate::style::Theme;
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let config = config::Config::load();
        (
            Self {
                state: AppState::Loading,
                color_theme: Some(config.theme),
            },
            Command::none(),
        )
    }
    fn theme(&self) -> Self::Theme {
        self.color_theme.unwrap()
    }
    fn title(&self) -> String {
        String::from("NostrTalk")
    }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let mut subscriptions = vec![];
        let backend_subscription = backend_connect().map(Message::BackEndEvent);

        let app_sub = match &self.state {
            AppState::Loaded { router, .. } => router.subscription().map(Message::RouterMessage),
            _ => iced::Subscription::none(),
        };

        let runtime_events = subscription::events().map(Message::RuntimeEvent);

        subscriptions.push(backend_subscription);
        subscriptions.push(app_sub);
        subscriptions.push(runtime_events);

        iced::Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            AppState::Loading => inform_card("Loading App", "Please wait...").into(),
            AppState::Loaded { router, .. } => router
                .view(self.color_theme)
                .map(Message::RouterMessage)
                .into(),
        }
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::RuntimeEvent(event) => {
                if let iced::Event::Window(window_event) = event {
                    if let window::Event::CloseRequested = window_event {
                        match &mut self.state {
                            AppState::Loading => {
                                return window::close();
                            }
                            AppState::Loaded {
                                conn,
                                already_sent_shutdown: shutdown_sent,
                                ..
                            } => {
                                tracing::info!("Shutting down backend");
                                if *shutdown_sent {
                                    return window::close();
                                } else {
                                    *shutdown_sent = true;
                                    if let Err(_e) = conn.send(net::ToBackend::Shutdown) {
                                        return window::close();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::RouterMessage(msg) => {
                if let AppState::Loaded { router, conn, .. } = &mut self.state {
                    match router.update(msg, conn) {
                        Ok(cmd) => return cmd.map(Message::RouterMessage),
                        Err(_e) => return window::close(),
                    }
                }
            }
            Message::BackEndEvent(event) => {
                tracing::trace!("Backend event: {:?}", event);

                if let BackendEvent::ThemeChanged(theme) = &event {
                    self.color_theme = Some(theme.to_owned());
                }

                match event {
                    BackendEvent::ShutdownDone => {
                        return window::close();
                    }
                    BackendEvent::Connected(mut conn) => {
                        let router = Router::new(&mut conn);
                        self.state = AppState::loaded(conn, router);
                    }
                    other => match &mut self.state {
                        AppState::Loaded { router, conn, .. } => {
                            match router.backend_event(other, conn) {
                                Ok(cmd) => return cmd.map(Message::RouterMessage),
                                Err(_e) => return window::close(),
                            }
                        }
                        _ => (),
                    },
                }
            }
        };

        Command::none()
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    setup_logger();

    App::run(Settings {
        exit_on_close_request: false,
        id: Some(String::from("nostrtalk")),
        window: window::Settings {
            size: (APP_WIDTH, APP_HEIGHT),
            min_size: Some((APP_MIN_WIDTH, APP_MIN_HEIGHT)),
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Default::default()
    })
    .expect("Failed to run app");
}

fn setup_logger() {
    // Cria um filtro de ambiente que define o nível de log padrão para todas as bibliotecas como ERROR e o nível de log do seu aplicativo como INFO
    let filter = EnvFilter::from_default_env()
        .add_directive("nostrtalk=info".parse().unwrap())
        .add_directive("warn".parse().unwrap());

    let subscriber = SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        // .with_writer(non_blocking)
        .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::default()) // Adicione esta linha para incluir eventos de spans
        .finish()
        .with(tracing_error::ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    tracing::info!("Starting up");
}

const APP_WIDTH: u32 = 800;
const APP_HEIGHT: u32 = 600;
const APP_MIN_WIDTH: u32 = 600;
const APP_MIN_HEIGHT: u32 = 400;
