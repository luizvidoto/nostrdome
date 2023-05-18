use futures::channel::mpsc;
use iced::widget::{button, container, row, text, Space};
use iced::Subscription;
use iced::{alignment, Command, Length};
use nostr_sdk::RelayStatus;

use crate::consts::NOSTRTALK_VERSION;
use crate::icon::signal_icon;
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub struct NetConnection(mpsc::UnboundedSender<Input>);
impl NetConnection {
    pub fn send(&mut self, input: Input) {
        if let Err(e) = self.0.unbounded_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Input {
    Wait,
    GetStatus(Vec<nostr_sdk::Relay>),
}
pub enum NetState {
    Initial,
    Idle {
        receiver: mpsc::UnboundedReceiver<Input>,
    },
    Querying {
        receiver: mpsc::UnboundedReceiver<Input>,
        relays: Vec<nostr_sdk::Relay>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Ready(NetConnection),
    Performing,
    Waited,
    UpdateNetStatus(usize),
    GoToAbout,
    GoToNetwork,
}
pub struct StatusBar {
    pub connected_relays: usize,
    pub sub_channel: Option<NetConnection>,
    pub client_relays: Option<Vec<nostr_sdk::Relay>>,
}
impl StatusBar {
    pub fn new() -> Self {
        Self {
            connected_relays: 0,
            sub_channel: None,
            client_relays: None,
        }
    }
    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match event {
            Event::GotRelayServers(relays) => {
                self.client_relays = Some(relays);
                self.send_action_to_channel(conn);
            }
            Event::RelayConnected(_) => {
                conn.send(net::Message::FetchRelayServers);
            }
            _ => (),
        }
        Command::none()
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Command<Message> {
        match message {
            Message::GoToAbout => (),
            Message::GoToNetwork => (),
            Message::Performing => {
                tracing::debug!("NetState Message:: Performing");
            }
            Message::Waited => {
                tracing::debug!("NetState Message:: Waited");
                self.send_action_to_channel(conn);
            }
            Message::Ready(channel) => {
                tracing::debug!("NetState Message:: Ready(channel)");
                self.sub_channel = Some(channel);
                self.send_action_to_channel(conn);
            }
            Message::UpdateNetStatus(connected_relays) => {
                self.connected_relays = connected_relays;
                if let (Some(ch), Some(relays)) = (&mut self.sub_channel, &self.client_relays) {
                    ch.send(Input::GetStatus(relays.clone()));
                }
            }
        }
        Command::none()
    }
    fn send_action_to_channel(&mut self, conn: &mut BackEndConnection) {
        if let Some(ch) = &mut self.sub_channel {
            let input_msg = match &self.client_relays {
                Some(relays) => Input::GetStatus(relays.clone()),
                None => {
                    conn.send(net::Message::FetchRelayServers);
                    Input::Wait
                }
            };
            ch.send(input_msg);
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        struct StatusBar;
        iced::subscription::unfold(
            std::any::TypeId::of::<StatusBar>(),
            NetState::Initial,
            |state| async move {
                match state {
                    NetState::Initial => {
                        let (sender, receiver) = mpsc::unbounded();
                        (
                            Message::Ready(NetConnection(sender)),
                            NetState::Idle { receiver },
                        )
                    }
                    NetState::Idle { mut receiver } => {
                        use iced::futures::StreamExt;

                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                        let input = receiver.select_next_some().await;

                        match input {
                            Input::GetStatus(relays) => {
                                (Message::Performing, NetState::Querying { receiver, relays })
                            }
                            Input::Wait => {
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                (Message::Waited, NetState::Idle { receiver })
                            }
                        }
                    }
                    NetState::Querying { relays, receiver } => {
                        let relays_connected = relays
                            .into_iter()
                            .filter(|r| {
                                futures::executor::block_on(r.status()) == RelayStatus::Connected
                            })
                            .count();
                        (
                            Message::UpdateNetStatus(relays_connected),
                            NetState::Idle { receiver },
                        )
                    }
                }
            },
        )
    }
    pub fn view(&self) -> Element<'static, Message> {
        let about = button(text(format!("NostrTalk v{}", NOSTRTALK_VERSION)).size(18))
            .padding([0, 2])
            .height(Length::Fill)
            .on_press(Message::GoToAbout)
            .style(style::Button::StatusBarButton);
        let signal = button(row![
            text(&self.connected_relays).size(18),
            signal_icon().size(12),
        ])
        .height(Length::Fill)
        .padding([0, 2])
        .on_press(Message::GoToNetwork)
        .style(style::Button::StatusBarButton);

        container(row![about, Space::with_width(Length::Fill), signal])
            .padding(0)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Bottom)
            .height(Length::Fixed(STATUS_BAR_HEIGHT))
            .width(Length::Fill)
            .style(style::Container::StatusBar)
            .into()
    }
}

pub const STATUS_BAR_HEIGHT: f32 = 20.0;
