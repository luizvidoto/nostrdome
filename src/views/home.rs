use iced::widget::{button, column, container, row, Space};
use iced::{alignment, Command, Length, Subscription};
use status_bar::StatusBar;

use crate::components::status_bar;
use crate::db::DbContact;
use crate::icon::settings_icon;
use crate::net::{BackEndConnection, BackendEvent};

use crate::widget::Text;
use crate::{
    icon::{home_icon, search_icon},
    style,
    widget::Element,
};

use super::{chat, find_channels, RouterCommand, RouterMessage};

#[derive(Debug, Clone)]
pub enum Message {
    DMsPressed,
    FindChannelsPressed,
    SettingsPressed,
    DmsMessage(chat::Message),
    FindChannelsMessage(find_channels::Message),
    StatusBarMessage(status_bar::Message),
}
pub struct State {
    active_view: ViewState,
    status_bar: StatusBar,
}
impl State {
    pub(crate) fn chat(conn: &mut BackEndConnection) -> Self {
        Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::DMs {
                state: chat::State::new(conn),
            },
        }
    }
    pub(crate) fn chat_to(db_contact: DbContact, conn: &mut BackEndConnection) -> Self {
        Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::DMs {
                state: chat::State::chat_to(db_contact, conn),
            },
        }
    }
    pub(crate) fn find_channels(conn: &mut BackEndConnection) -> State {
        Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::FindChannel {
                state: find_channels::State::new(conn),
            },
        }
    }
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            self.active_view.subscription(),
            self.status_bar
                .subscription()
                .map(Message::StatusBarMessage),
        ])
    }
    pub(crate) fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let mut commands = RouterCommand::new();

        let cmd = self.status_bar.backend_event(event.clone(), conn);
        commands.push(cmd.map(Message::StatusBarMessage));

        let (cmds, router_message) = self.active_view.backend_event(event.clone(), conn).batch();
        if let Some(router_message) = router_message {
            commands.change_route(router_message);
        }
        commands.push(cmds);

        commands
    }
    pub(crate) fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let mut commands = RouterCommand::new();

        match message {
            Message::DMsPressed => {
                self.active_view = ViewState::DMs {
                    state: chat::State::new(conn),
                }
            }
            Message::FindChannelsPressed => {
                self.active_view = ViewState::FindChannel {
                    state: find_channels::State::new(conn),
                }
            }
            Message::StatusBarMessage(status_msg) => {
                let (cmd, router_message) = self.status_bar.update(status_msg, conn);
                if let Some(router_message) = router_message {
                    commands.change_route(router_message);
                }
                commands.push(cmd.map(Message::StatusBarMessage));
            }
            Message::SettingsPressed => commands.change_route(RouterMessage::GoToSettings),
            other => {
                let (cmds, msg) = self.active_view.update(other, conn).batch();
                if let Some(msg) = msg {
                    commands.change_route(msg)
                }
                commands.push(cmds);
            }
        }

        commands
    }
    pub fn view(&self) -> Element<Message> {
        // like discord, on the left

        let dm_btn = make_menu_btn(self.active_view.is_dms(), home_icon, Message::DMsPressed);
        let find_ch_btn = make_menu_btn(
            self.active_view.is_find_channel(),
            search_icon,
            Message::FindChannelsPressed,
        );
        let settings_btn = make_menu_btn(false, settings_icon, Message::SettingsPressed);
        let nav_bar = container(
            column![
                dm_btn,
                find_ch_btn,
                Space::with_height(Length::Fill),
                settings_btn
            ]
            .spacing(2),
        )
        .padding([2, 0])
        .style(style::Container::ChatContainer)
        .width(NAVBAR_WIDTH)
        .height(Length::Fill);

        let status_bar = self.status_bar.view().map(Message::StatusBarMessage);

        let active_view = column![
            container(self.active_view.view())
                .width(Length::Fill)
                .height(Length::Fill),
            status_bar
        ];

        row![nav_bar, active_view].into()
    }
}

fn make_menu_btn<'a, M: 'a + Clone>(
    is_active: bool,
    icon: impl Fn() -> Text<'a>,
    message: M,
) -> Element<'a, M> {
    let style = if is_active {
        style::Button::ActiveMenuBtn
    } else {
        style::Button::MenuBtn
    };

    container(
        button(
            icon()
                .size(ICON_SIZE)
                .width(Length::Fill)
                .height(Length::Fill)
                .vertical_alignment(alignment::Vertical::Center)
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .style(style)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(message),
    )
    .padding([PADDING_V, PADDING_H])
    .width(NAVBAR_WIDTH)
    .height(NAVBAR_WIDTH)
    .into()
}

pub enum ViewState {
    DMs { state: chat::State },
    FindChannel { state: find_channels::State },
}
impl ViewState {
    pub(crate) fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let mut commands = RouterCommand::new();
        match message {
            Message::DmsMessage(dms_msg) => {
                if let ViewState::DMs { state } = self {
                    let (cmds, msg) = state.update(dms_msg, conn).batch();
                    if let Some(msg) = msg {
                        commands.change_route(msg)
                    }
                    commands.push(cmds.map(Message::DmsMessage));
                }
            }
            Message::FindChannelsMessage(find_ch_msg) => {
                if let ViewState::FindChannel { state } = self {
                    let (cmds, msg) = state.update(find_ch_msg, conn).batch();
                    if let Some(msg) = msg {
                        commands.change_route(msg)
                    }
                    commands.push(cmds.map(Message::FindChannelsMessage));
                }
            }
            Message::StatusBarMessage(_) => (),
            Message::SettingsPressed => (),
            Message::FindChannelsPressed => (),
            Message::DMsPressed => (),
        }

        commands
    }
    pub(crate) fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let mut commands = RouterCommand::new();

        match self {
            ViewState::DMs { state } => {
                let (cmds, msg) = state.backend_event(event, conn).batch();
                if let Some(msg) = msg {
                    commands.change_route(msg);
                }
                commands.push(cmds.map(Message::DmsMessage));
            }
            ViewState::FindChannel { state } => {
                let (cmds, msg) = state.backend_event(event, conn).batch();
                if let Some(msg) = msg {
                    commands.change_route(msg);
                }
                commands.push(cmds.map(Message::FindChannelsMessage));
            }
        }

        commands
    }
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        match self {
            ViewState::DMs { state } => state.subscription().map(Message::DmsMessage),
            ViewState::FindChannel { state } => {
                state.subscription().map(Message::FindChannelsMessage)
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            ViewState::DMs { state } => state.view().map(Message::DmsMessage),
            ViewState::FindChannel { state } => state.view().map(Message::FindChannelsMessage),
        }
    }
    pub fn is_dms(&self) -> bool {
        match self {
            ViewState::DMs { .. } => true,
            _ => false,
        }
    }
    pub fn is_find_channel(&self) -> bool {
        match self {
            ViewState::FindChannel { .. } => true,
            _ => false,
        }
    }
}

const NAVBAR_WIDTH: u16 = 55;
const PADDING_V: u16 = 6;
const PADDING_H: u16 = 6;
const ICON_SIZE: u16 = 26;
