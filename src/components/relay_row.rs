use crate::error::Error;
use crate::types::RelayUrl;
use iced::widget::{button, checkbox, container, row, text};
use iced::{Command, Element, Length, Subscription};
use iced_native::futures::channel::mpsc;
use nostr_sdk::{Relay, RelayStatus};

#[derive(Debug, Clone)]
pub struct RelayRowConnection(mpsc::UnboundedSender<Input>);
impl RelayRowConnection {
    pub fn send(&mut self, input: Input) {
        if let Err(e) = self.0.unbounded_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    UpdateStatus((RelayUrl, RelayStatus)),
    DeleteRelay(RelayUrl),
    ToggleRead,
    ToggleWrite,
    ToggleAdvertise,
    Ready(RelayRowConnection),
}

pub enum Input {
    GetStatus,
}
pub enum State {
    Idle {
        url: RelayUrl,
        relay: Relay,
    },
    Querying {
        url: RelayUrl,
        receiver: mpsc::UnboundedReceiver<Input>,
        relay: Relay,
    },
}

#[derive(Debug, Clone)]
pub struct RelayRow {
    status: RelayStatus,
    url: RelayUrl,
    last_connected_at: i64,
    is_read: bool,
    is_write: bool,
    is_advertise: bool,
    relay: Relay,
}

impl RelayRow {
    pub fn subscription(&self) -> Subscription<Message> {
        struct RelayRowSub;
        let id = std::any::TypeId::of::<RelayRowSub>();
        iced::subscription::unfold(
            id,
            State::Idle {
                url: self.url.clone(),
                relay: self.relay.clone(),
            },
            |state| async move {
                match state {
                    State::Idle { relay, url } => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        let (sender, receiver) = mpsc::unbounded();
                        (
                            Message::Ready(RelayRowConnection(sender)),
                            State::Querying {
                                url,
                                relay,
                                receiver,
                            },
                        )
                    }
                    State::Querying {
                        url,
                        relay,
                        mut receiver,
                    } => {
                        use iced_native::futures::StreamExt;

                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let input = receiver.select_next_some().await;

                        let status = match input {
                            Input::GetStatus => relay.status().await,
                        };

                        (
                            Message::UpdateStatus((url.clone(), status)),
                            State::Idle { url, relay },
                        )
                    }
                }
            },
        )
    }
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UpdateStatus((url, status)) => {
                if self.url == url {
                    self.status = status;
                }
            }
            _ => (),
        }
        Command::none()
    }
    pub fn new(relay: Relay) -> Result<Self, Error> {
        let url = RelayUrl::try_from_str(relay.url().as_str())?;
        Ok(Self {
            status: RelayStatus::Disconnected,
            url,
            last_connected_at: relay.stats().connected_at().as_i64(),
            is_read: relay.opts().read(),
            is_write: relay.opts().write(),
            is_advertise: false,
            relay,
        })
    }
    fn seconds_since_last_conn(&self) -> i64 {
        let now = chrono::Utc::now().timestamp_millis() / 1000;
        now - self.last_connected_at
    }
    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        row![
            text(&self.status).width(Length::Fill),
            container(text(&self.url)).width(Length::Fill),
            container(text(format!("{}s", self.seconds_since_last_conn()))).width(Length::Fill),
            container(checkbox("", self.is_read, |_| Message::ToggleRead)).width(Length::Fill),
            container(checkbox("", self.is_write, |_| { Message::ToggleWrite }))
                .width(Length::Fill),
            container(checkbox("", self.is_advertise, |_| {
                Message::ToggleAdvertise
            }))
            .width(Length::Fill),
            button("Remove")
                .on_press(Message::DeleteRelay(self.url.clone()))
                .width(Length::Fill),
        ]
        .into()
    }
    pub fn view_header() -> Element<'static, Message> {
        row![
            text("Status").width(Length::Fill),
            text("Address").width(Length::Fill),
            text("Last Active").width(Length::Fill),
            text("Read").width(Length::Fill),
            text("Write").width(Length::Fill),
            text("Advertise").width(Length::Fill),
            text("").width(Length::Fill)
        ]
        .into()
    }
}

// impl From<&RelayRow> for DbRelay {
//     fn from(row: &RelayRow) -> Self {
//         Self {
//             url: row.url.to_owned(),
//             last_connected_at: row.last_connected_at,
//             read: row.is_read,
//             write: row.is_write,
//             advertise: row.is_advertise,
//         }
//     }
// }
