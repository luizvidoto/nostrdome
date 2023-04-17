use iced::widget::{button, column, container, row};
use iced::{Color, Element, Length};
use nostr_sdk::Metadata;

use crate::net::database::DbConnection;
use crate::net::nostr::NostrConnection;
use crate::net::{self};

mod account;
mod appearance;
mod backup;
mod contacts;
mod network;

#[derive(Debug, Clone)]
pub enum Message {
    AccountMessage(account::Message),
    AppearanceMessage(appearance::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    ContactsMessage(contacts::Message),
    MenuAccountPress,
    MenuAppearancePress,
    MenuNetworkPress,
    MenuBackupPress,
    MenuContactsPress,
    NavEscPress,
}

#[derive(Debug, Clone)]
pub enum State {
    Account { state: account::State },
    Appearance { state: appearance::State },
    Network { state: network::State },
    Backup { state: backup::State },
    Contacts { state: contacts::State },
}
impl State {
    pub fn new(db_conn: &mut DbConnection) -> Self {
        Self::account(db_conn)
    }
    fn account(_db_conn: &mut DbConnection) -> Self {
        let profile = Metadata::new();
        // conn.send(message)
        Self::Account {
            state: account::State::new(profile),
        }
    }
    fn appearance() -> Self {
        Self::Appearance {
            state: appearance::State::new(),
        }
    }
    fn network_loading(db_conn: &mut DbConnection) -> Self {
        // let mut relays = vec![];
        // for _ in 0..10 {
        //     relays.push(RelayRow::new((8..12).fake::<String>()))
        // }
        Self::Network {
            state: network::State::loading(db_conn),
        }
    }
    fn contacts(db_conn: &mut DbConnection) -> Self {
        Self::Contacts {
            state: contacts::State::loading(db_conn),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn db_event(
        &mut self,
        event: net::database::Event,
        db_conn: &mut DbConnection,
        _nostr_conn: &mut NostrConnection,
    ) {
        match self {
            Self::Account { state } => state.update(account::Message::DbEvent(event)),
            Self::Appearance { state } => state.update(appearance::Message::DbEvent(event)),
            Self::Network { state } => state.update(network::Message::DbEvent(event), db_conn),
            Self::Backup { state } => state.update(backup::Message::DbEvent(event)),
            Self::Contacts { state } => state.update(contacts::Message::DbEvent(event), db_conn),
        }
    }
    pub fn update(&mut self, message: Message, db_conn: &mut DbConnection) {
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
                    state.update(msg, db_conn);
                }
            }
            Message::BackupMessage(msg) => {
                if let State::Backup { state } = self {
                    state.update(msg);
                }
            }
            Message::ContactsMessage(msg) => {
                if let State::Contacts { state } = self {
                    state.update(msg, db_conn);
                }
            }
            Message::NavEscPress => (),
            Message::MenuAccountPress => {
                *self = Self::account(db_conn);
            }
            Message::MenuAppearancePress => {
                *self = Self::appearance();
            }
            Message::MenuNetworkPress => {
                *self = Self::network_loading(db_conn);
            }
            Message::MenuBackupPress => {
                *self = Self::backup();
            }
            Message::MenuContactsPress => {
                *self = Self::contacts(db_conn);
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let view = match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance { state } => state.view().map(Message::AppearanceMessage),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
            Self::Contacts { state } => state.view().map(Message::ContactsMessage),
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
        let contacts_btn = button("Contacts")
            .style(match self {
                Self::Contacts { .. } => iced::theme::Button::Custom(Box::new(ActiveMenuBtn {})),
                _ => iced::theme::Button::Custom(Box::new(InactiveMenuBtn {})),
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuContactsPress);
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
                backup_btn,
                contacts_btn
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
