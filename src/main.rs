pub mod components;
pub mod error;
pub mod net;
pub mod types;
pub mod views;

use iced::{executor, widget::text, window, Application, Command, Element, Settings};
use net::{nostr_connect, Connection};
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
            Message::NostrClientMessage(nostr_event) => {
                // println!("Event from nostr client: {:?}", &nostr_event);
                match nostr_event {
                    net::Event::Connected(conn) => self.state = State::loaded(conn),
                    net::Event::Disconnected => {}
                    net::Event::Error(error_msg) => {
                        println!("Error: {}", error_msg);
                    }
                    net::Event::GotRelays(relays) => {
                        for r in relays {
                            println!("{}", r.url());
                        }
                    }
                }
            }
        }

        Command::none()
    }
}

#[tokio::main]
async fn main() {
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
