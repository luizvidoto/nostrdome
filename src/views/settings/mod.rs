use iced::widget::{button, column, container, row};
use iced::{Command, Length, Subscription};
use nostr_sdk::Metadata;

use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::Element;

mod account;
pub mod appearance;
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

#[derive(Debug)]
pub enum State {
    Account { state: account::State },
    Appearance { state: appearance::State },
    Network { state: network::State },
    Backup { state: backup::State },
    Contacts { state: contacts::State },
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            State::Network { state } => state.subscription().map(Message::NetworkMessage),
            _ => Subscription::none(),
        }
    }
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        Self::account(back_conn)
    }
    fn account(_back_conn: &mut BackEndConnection) -> Self {
        let profile = Metadata::new();
        // conn.send(message)
        Self::Account {
            state: account::State::new(profile),
        }
    }
    fn appearance(selected_theme: Option<style::Theme>) -> Self {
        Self::Appearance {
            state: appearance::State::new(selected_theme),
        }
    }
    fn network(back_conn: &mut BackEndConnection) -> Self {
        Self::Network {
            state: network::State::new(back_conn),
        }
    }
    pub fn contacts(back_conn: &mut BackEndConnection) -> Self {
        Self::Contacts {
            state: contacts::State::new(back_conn),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn back_end_event(
        &mut self,
        event: net::Event,
        back_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match self {
            Self::Account { state } => state.update(account::Message::BackEndEvent(event)),
            Self::Appearance { state } => state.update(appearance::Message::BackEndEvent(event)),
            Self::Network { state } => {
                return state
                    .update(network::Message::BackEndEvent(event), back_conn)
                    .map(Message::NetworkMessage);
            }
            Self::Backup { state } => state.update(backup::Message::BackEndEvent(event)),
            Self::Contacts { state } => {
                state.update(contacts::Message::BackEndEvent(event), back_conn)
            }
        }
        Command::none()
    }
    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> Command<Message> {
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
                    return state.update(msg, back_conn).map(Message::NetworkMessage);
                }
            }
            Message::BackupMessage(msg) => {
                if let State::Backup { state } = self {
                    state.update(msg);
                }
            }
            Message::ContactsMessage(msg) => {
                if let State::Contacts { state } = self {
                    state.update(msg, back_conn);
                }
            }
            Message::NavEscPress => (),
            Message::MenuAccountPress => {
                *self = Self::account(back_conn);
            }
            Message::MenuAppearancePress => {
                *self = Self::appearance(selected_theme);
            }
            Message::MenuNetworkPress => {
                *self = Self::network(back_conn);
            }
            Message::MenuBackupPress => {
                *self = Self::backup();
            }
            Message::MenuContactsPress => {
                *self = Self::contacts(back_conn);
            }
        }
        Command::none()
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
                Self::Account { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAccountPress);
        let appearance_btn = button("Appearance")
            .style(match self {
                Self::Appearance { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuAppearancePress);
        let network_btn = button("Network")
            .style(match self {
                Self::Network { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuNetworkPress);
        let backup_btn = button("Backup")
            .style(match self {
                Self::Backup { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuBackupPress);
        let contacts_btn = button("Contacts")
            .style(match self {
                Self::Contacts { .. } => style::Button::ActiveMenuBtn,
                _ => style::Button::InactiveMenuBtn,
            })
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::MenuContactsPress);
        let esc_btn = button("Esc")
            .padding(10)
            .on_press(Message::NavEscPress)
            .style(style::Button::InactiveMenuBtn);
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
