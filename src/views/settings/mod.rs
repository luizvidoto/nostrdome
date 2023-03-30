use fake::Fake;
use iced::widget::{button, column, container, row};
use iced::{Color, Element, Length};

use self::network::RelayRow;

mod account;
mod appearance;
mod backup;
mod network;

#[derive(Debug, Clone)]
pub enum Message {
    AccountMessage(account::Message),
    AppearanceMessage(appearance::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    MenuAccountPress,
    MenuAppearancePress,
    MenuNetworkPress,
    MenuBackupPress,
    NavEscPress,
}

#[derive(Debug, Clone)]
pub enum State {
    Account { state: account::State },
    Appearance { state: appearance::State },
    Network { state: network::State },
    Backup { state: backup::State },
}
impl Default for State {
    fn default() -> Self {
        Self::account()
    }
}
impl State {
    fn account() -> Self {
        Self::Account {
            state: account::State::default(),
        }
    }
    fn appearance() -> Self {
        Self::Appearance {
            state: appearance::State::default(),
        }
    }
    fn network() -> Self {
        let mut relays = vec![];
        for _ in 0..10 {
            relays.push(RelayRow::new((8..12).fake::<String>()))
        }
        Self::Network {
            state: network::State::new(relays),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn new() -> Self {
        Self::account()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AccountMessage(msg) => {
                if let State::Account { state } = self {
                    state.update(msg);
                }
            }
            Message::AppearanceMessage(msg) => {
                if let State::Appearance { state } = self {
                    state.update(msg);
                }
            }
            Message::NetworkMessage(msg) => {
                if let State::Network { state } = self {
                    state.update(msg);
                }
            }
            Message::BackupMessage(msg) => {
                if let State::Backup { state } = self {
                    state.update(msg);
                }
            }
            Message::NavEscPress => (),
            Message::MenuAccountPress => {
                *self = Self::account();
            }
            Message::MenuAppearancePress => {
                *self = Self::appearance();
            }
            Message::MenuNetworkPress => {
                *self = Self::network();
            }
            Message::MenuBackupPress => {
                *self = Self::backup();
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let view = match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance { state } => state.view().map(Message::AppearanceMessage),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
        };

        let account_btn = button("Account")
            .style(match self {
                Self::Account { .. } => iced::theme::Button::Custom(Box::new(ActiveMenuBtn {})),
                _ => iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})),
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAccountPress);
        let appearance_btn = button("Appearance")
            .style(match self {
                Self::Appearance { .. } => iced::theme::Button::Custom(Box::new(ActiveMenuBtn {})),
                _ => iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})),
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAppearancePress);
        let network_btn = button("Network")
            .style(match self {
                Self::Network { .. } => iced::theme::Button::Custom(Box::new(ActiveMenuBtn {})),
                _ => iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})),
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuNetworkPress);
        let backup_btn = button("Backup")
            .style(match self {
                Self::Backup { .. } => iced::theme::Button::Custom(Box::new(ActiveMenuBtn {})),
                _ => iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})),
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuBackupPress);
        let esc_btn = button("Esc")
            .padding(10)
            .on_press(Message::NavEscPress)
            .style(iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})));
        let spacer = iced::widget::horizontal_space(Length::Fixed(3.0));
        let menubar = container(
            column![
                esc_btn,
                spacer,
                account_btn,
                appearance_btn,
                network_btn,
                backup_btn
            ]
            .spacing(3),
        )
        .width(Length::FillPortion(1))
        .padding([10, 5]);
        let view_ct = container(view)
            .padding([10, 5])
            .width(Length::FillPortion(3));
        let content = row![menubar, view_ct]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct InactiveMenuBtn;
impl button::StyleSheet for InactiveMenuBtn {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: Color::BLACK,
            border_radius: 5.0,
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Color::from_rgb8(240, 240, 240).into()),
            text_color: Color::BLACK,
            ..self.active(style)
        }
    }
}

struct ActiveMenuBtn;
impl button::StyleSheet for ActiveMenuBtn {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: Color::WHITE,
            border_radius: 5.0,
            background: Some(Color::from_rgb8(65, 159, 217).into()),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            ..self.active(style)
        }
    }
}
