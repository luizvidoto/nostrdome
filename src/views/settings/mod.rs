use iced::widget::{button, column, container, row, text};
use iced::{theme, Element, Length};

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
        Self::Network {
            state: network::State::default(),
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
            Message::AccountMessage(_) => todo!(),
            Message::AppearanceMessage(_) => todo!(),
            Message::NetworkMessage(_) => todo!(),
            Message::BackupMessage(_) => todo!(),
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
                Self::Account { .. } => theme::Button::Primary,
                _ => theme::Button::Secondary,
            })
            .padding(10)
            .on_press(Message::MenuAccountPress);
        let appearance_btn = button("Appearance")
            .style(match self {
                Self::Appearance { .. } => theme::Button::Primary,
                _ => theme::Button::Secondary,
            })
            .padding(10)
            .on_press(Message::MenuAppearancePress);
        let network_btn = button("Network")
            .style(match self {
                Self::Network { .. } => theme::Button::Primary,
                _ => theme::Button::Secondary,
            })
            .padding(10)
            .on_press(Message::MenuNetworkPress);
        let backup_btn = button("Backup")
            .style(match self {
                Self::Backup { .. } => theme::Button::Primary,
                _ => theme::Button::Secondary,
            })
            .padding(10)
            .on_press(Message::MenuBackupPress);

        let menubar = column![account_btn, appearance_btn, network_btn, backup_btn]
            .width(Length::Fill)
            .padding(10)
            .spacing(10);

        let content = row![menubar, view].width(Length::Fill).height(Length::Fill);

        let esc_btn = button("Esc").padding(10).on_press(Message::NavEscPress);
        let empty = container(text("")).width(Length::Fill);
        let topbar = row![empty, esc_btn]
            .width(Length::Fill)
            .padding(10)
            .spacing(10);

        column![topbar, content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
