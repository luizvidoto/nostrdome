pub mod components;
pub mod db;
pub mod error;
pub mod net;
pub mod types;
pub mod utils;
pub mod views;

use components::text::title;
use iced::{
    executor,
    widget::{button, column, container, text},
    window, Application, Command, Element, Length, Settings,
};
use net::{
    database::{database_connect, DbConnection},
    nostr::{nostr_connect, NostrConnection},
};
use nostr_sdk::{prelude::FromSkStr, Keys};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use views::{login, Router};

const IN_MEMORY: bool = true;

#[derive(Debug, Clone)]
pub enum Message {
    RouterMessage(views::Message),
    DatabaseMessage(net::database::Event),
    NostrClientMessage(net::nostr::Event),
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
    LoadedDB {
        keys: Keys,
        db_conn: DbConnection,
    },
    LoadedAll {
        keys: Keys,
        db_conn: DbConnection,
        nostr_conn: NostrConnection,
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
    pub fn loaded_db(keys: Keys, db_conn: DbConnection) -> Self {
        Self::LoadedDB { keys, db_conn }
    }
    pub fn loaded_all(keys: Keys, mut db_conn: DbConnection, nostr_conn: NostrConnection) -> Self {
        let router = Router::new(&keys, &mut db_conn);
        Self::LoadedAll {
            keys,
            db_conn,
            nostr_conn,
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
    type Theme = iced::Theme;
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
        let nostr_subscription = match &self.state {
            State::Login { .. } | State::OutsideError { .. } | State::Loading { .. } => {
                iced::Subscription::none()
            }
            State::LoadedDB { keys, .. } | State::LoadedAll { keys, .. } => {
                nostr_connect(keys.to_owned()).map(Message::NostrClientMessage)
            }
        };
        let database_subscription = match &self.state {
            State::Login { .. } | State::OutsideError { .. } => iced::Subscription::none(),
            State::Loading { keys, .. }
            | State::LoadedDB { keys, .. }
            | State::LoadedAll { keys, .. } => {
                database_connect(IN_MEMORY, &keys.public_key().to_string())
                    .map(Message::DatabaseMessage)
            }
        };
        iced::Subscription::batch(vec![database_subscription, nostr_subscription])
    }
    fn view(&self) -> Element<Self::Message> {
        let content: Element<_> = match &self.state {
            State::Login { state } => state.view().map(Message::LoginMessage),
            State::Loading { .. } => text("Loading database...").into(),
            State::LoadedDB { .. } => text("Loading nostr...").into(),
            State::LoadedAll { router, .. } => router.view().map(Message::RouterMessage),
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
                if let State::LoadedAll {
                    db_conn,
                    router,
                    nostr_conn,
                    ..
                } = &mut self.state
                {
                    router.update(msg, db_conn, nostr_conn);
                }
            }
            Message::DatabaseMessage(db_ev) => match db_ev {
                net::database::Event::Connected(db_conn) => {
                    if let State::Loading { keys } = &mut self.state {
                        self.state = State::loaded_db(keys.clone(), db_conn);
                    }
                    println!("Received Database Connected Event");
                }
                net::database::Event::Disconnected => {
                    self.state = State::login();
                }
                net::database::Event::Error(e) => {
                    tracing::error!("{}", e);
                }
                ev => {
                    if let State::LoadedAll {
                        router,
                        db_conn,
                        nostr_conn,
                        ..
                    } = &mut self.state
                    {
                        router.db_event(ev, db_conn, nostr_conn);
                    }
                }
            },
            Message::NostrClientMessage(nostr_ev) => match &mut self.state {
                State::LoadedDB { keys, db_conn } => {
                    if let net::nostr::Event::Connected(mut nostr_conn) = nostr_ev {
                        if let Err(e) = nostr_conn.send(net::nostr::Message::ConnectRelays) {
                            tracing::error!("{}", e);
                        }
                        self.state =
                            State::loaded_all(keys.to_owned(), db_conn.to_owned(), nostr_conn);
                    }
                }
                State::LoadedAll {
                    db_conn,
                    nostr_conn,
                    router,
                    keys,
                    ..
                } => {
                    router.nostr_event(nostr_ev, &keys, db_conn, nostr_conn);
                }
                _ => {
                    tracing::error!("Not possible!");
                    tracing::error!("{:?}", nostr_ev);
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

// net::Event::SomeEventSuccessId(ev_id) => {
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
