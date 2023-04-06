pub mod components;
pub mod db;
pub mod error;
pub mod net;
pub mod types;
pub mod utils;
pub mod views;

use iced::{executor, widget::text, window, Application, Command, Element, Settings};
use net::{nostr_connect, Connection};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use views::Router;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    NostrClientMessage(net::Event),
}
#[derive(Debug)]
pub enum State {
    Loading,
    Loaded { conn: Connection, router: Router },
    Error,
}
impl State {
    pub fn loaded(conn: Connection) -> Self {
        Self::Loaded {
            conn,
            router: Router::default(),
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
    type Theme = iced::Theme;
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                state: State::Loading,
            },
            Command::none(),
        )
    }
    fn title(&self) -> String {
        String::from("NostrDome")
    }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        nostr_connect().map(Message::NostrClientMessage)
    }
    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            State::Loading => text("Loading...").into(),
            State::Loaded { router, .. } => router.view().map(Message::RouterMessage),
            State::Error => text("Error loading database.").into(),
        }
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::RouterMessage(msg) => {
                if let State::Loaded { conn, router } = &mut self.state {
                    router.update(msg, conn);
                }
            }
            Message::NostrClientMessage(nostr_event) => match nostr_event {
                net::Event::Connected(conn) => self.state = State::loaded(conn),
                net::Event::Disconnected => {}
                net::Event::Error(e) => {
                    tracing::error!("{}", e);
                }
                ev => {
                    if let State::Loaded { conn, router } = &mut self.state {
                        router.update(views::Message::DbEvent(ev), conn);
                    }
                } // net::Event::SomeEventSuccessId(ev_id) => {
                  //     println!("Success! Event id: {}", ev_id);
                  // }
                  // net::Event::GotPublicKey(pb_key) => {
                  //     println!("Public key: {}", pb_key);
                  // }
                  // net::Event::DirectMessage(msg) => {
                  //     println!("New DM {}", msg);
                  // }
                  // net::Event::NostrEvent(event) => {
                  //     println!("{:?}", event);
                  // }
                  // net::Event::GotRelays(relays) => {
                  //     for r in relays {
                  //         println!("{}", r.url());
                  //     }
                  // }
                  // net::Event::GotOwnEvents(events) => {
                  //     println!("Got Own events");
                  //     for (idx, e) in events.iter().enumerate() {
                  //         println!("{}: {:?}", idx, e)
                  //     }
                  // }
            },
        }

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
            size: (600, 800),
            min_size: Some((600, 400)),
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        ..Default::default()
    })
    .expect("Failed to run app");
}
