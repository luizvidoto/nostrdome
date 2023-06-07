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

use components::inform_card;
use dotenv::dotenv;
use iced::{executor, window, Application, Command, Settings};
use net::{backend_connect, BackEndConnection, BackendEvent};
// use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::SubscriberBuilder, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};
use views::{
    settings::{self, appearance},
    Router,
};
use widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    BackEndEvent(BackendEvent),
}
pub enum AppState {
    Loading,
    Loaded {
        conn: BackEndConnection,
        router: Router,
    },
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
        (
            Self {
                state: AppState::Loading,
                color_theme: Some(style::Theme::default()),
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

        subscriptions.push(backend_subscription);
        subscriptions.push(app_sub);

        iced::Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            AppState::Loading => inform_card("Loading App", "Please wait...").into(),
            AppState::Loaded { router, .. } => router.view().map(Message::RouterMessage).into(),
        }
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::RouterMessage(msg) => {
                if let AppState::Loaded { router, conn } = &mut self.state {
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
            Message::BackEndEvent(event) => {
                tracing::trace!("Backend event: {:?}", event);
                match event {
                    BackendEvent::Connected(mut conn) => {
                        let router = Router::new(&mut conn);
                        self.state = AppState::Loaded { conn, router };
                    }
                    other => match &mut self.state {
                        AppState::Loaded { router, conn, .. } => {
                            return router
                                .backend_event(other, conn)
                                .map(Message::RouterMessage);
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

    App::run(Settings {
        id: Some(String::from("nostrtalk")),
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
