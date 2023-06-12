use iced::widget::{button, column, container, row, Rule, Space};
use iced::{alignment, Length, Subscription};
use status_bar::StatusBar;

use crate::components::{common_scrollable, invisible_scrollable, status_bar};
use crate::db::DbContact;
use crate::icon::{settings_icon, wand_icon};
use crate::net::{BackEndConnection, BackendEvent};

use crate::widget::Text;
use crate::{
    icon::{home_icon, search_icon},
    style,
    widget::Element,
};

use super::route::Route;
use super::{chat, find_channels, theme_styling, RouterCommand, RouterMessage};

#[derive(Debug, Clone)]
pub enum Message {
    DMsPressed,
    FindChannelsPressed,
    SettingsPressed,
    ThemeStylingMsg(theme_styling::Message),
    DmsMessage(chat::Message),
    FindChannelsMessage(find_channels::Message),
    StatusBarMessage(status_bar::Message),
    ThemeStylingPressed,
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
}
impl Route for State {
    type Message = Message;
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            self.active_view.subscription(),
            self.status_bar
                .subscription()
                .map(Message::StatusBarMessage),
        ])
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        let mut commands = self.active_view.backend_event(event.clone(), conn);

        let cmd = self.status_bar.backend_event(event.clone(), conn);
        commands.push(cmd.map(Message::StatusBarMessage));

        commands
    }
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        let mut commands = RouterCommand::new();

        match message {
            Message::ThemeStylingPressed => match self.active_view {
                ViewState::ThemeStyling { .. } => (),
                _ => {
                    self.active_view = ViewState::ThemeStyling {
                        state: theme_styling::State::new(conn),
                    }
                }
            },
            Message::DMsPressed => match self.active_view {
                ViewState::DMs { .. } => (),
                _ => {
                    self.active_view = ViewState::DMs {
                        state: chat::State::new(conn),
                    }
                }
            },
            Message::FindChannelsPressed => match self.active_view {
                ViewState::FindChannel { .. } => (),
                _ => {
                    self.active_view = ViewState::FindChannel {
                        state: find_channels::State::new(conn),
                    }
                }
            },
            Message::StatusBarMessage(status_msg) => {
                return self
                    .status_bar
                    .update(status_msg, conn)
                    .map(Message::StatusBarMessage);
            }
            Message::SettingsPressed => commands.change_route(RouterMessage::GoToSettings),
            other => {
                return self.active_view.update(other, conn);
            }
        }

        commands
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        let theme_styling_btn = make_menu_btn(
            self.active_view.is_theme_styling(),
            wand_icon,
            Message::ThemeStylingPressed,
        );
        let dm_btn = make_menu_btn(self.active_view.is_dms(), home_icon, Message::DMsPressed);
        let find_ch_btn = make_menu_btn(
            self.active_view.is_find_channel(),
            search_icon,
            Message::FindChannelsPressed,
        );
        let settings_btn = make_menu_btn(false, settings_icon, Message::SettingsPressed);
        let spacer = container(Rule::horizontal(2))
            .padding([0, 10])
            .width(Length::Fill);

        let nav_bar = container(
            column![
                container(invisible_scrollable(
                    column![dm_btn, spacer, find_ch_btn, theme_styling_btn,].spacing(2)
                ),)
                .height(Length::Fill),
                settings_btn,
            ]
            .spacing(2),
        )
        .padding([2, 0])
        .style(style::Container::ChatContainer)
        .width(NAVBAR_WIDTH)
        .height(Length::Fill);

        let status_bar = self.status_bar.view().map(Message::StatusBarMessage);

        let active_view = column![
            container(self.active_view.view(selected_theme))
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
    ThemeStyling { state: theme_styling::State },
    DMs { state: chat::State },
    FindChannel { state: find_channels::State },
}
impl ViewState {
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

    fn is_theme_styling(&self) -> bool {
        match self {
            ViewState::ThemeStyling { .. } => true,
            _ => false,
        }
    }
}
impl Route for ViewState {
    type Message = Message;
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        let commands = RouterCommand::new();
        match message {
            Message::ThemeStylingMsg(dms_msg) => {
                if let ViewState::ThemeStyling { state } = self {
                    return state.update(dms_msg, conn).map(Message::ThemeStylingMsg);
                }
            }
            Message::DmsMessage(dms_msg) => {
                if let ViewState::DMs { state } = self {
                    return state.update(dms_msg, conn).map(Message::DmsMessage);
                }
            }
            Message::FindChannelsMessage(find_ch_msg) => {
                if let ViewState::FindChannel { state } = self {
                    return state
                        .update(find_ch_msg, conn)
                        .map(Message::FindChannelsMessage);
                }
            }
            Message::StatusBarMessage(_) => (),
            Message::ThemeStylingPressed => (),
            Message::SettingsPressed => (),
            Message::FindChannelsPressed => (),
            Message::DMsPressed => (),
        }

        commands
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        match self {
            ViewState::ThemeStyling { state } => state
                .backend_event(event, conn)
                .map(Message::ThemeStylingMsg),
            ViewState::DMs { state } => state.backend_event(event, conn).map(Message::DmsMessage),
            ViewState::FindChannel { state } => state
                .backend_event(event, conn)
                .map(Message::FindChannelsMessage),
        }
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        match self {
            ViewState::ThemeStyling { state } => state.subscription().map(Message::ThemeStylingMsg),
            ViewState::DMs { state } => state.subscription().map(Message::DmsMessage),
            ViewState::FindChannel { state } => {
                state.subscription().map(Message::FindChannelsMessage)
            }
        }
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        match self {
            ViewState::ThemeStyling { state } => {
                state.view(selected_theme).map(Message::ThemeStylingMsg)
            }
            ViewState::DMs { state } => state.view(selected_theme).map(Message::DmsMessage),
            ViewState::FindChannel { state } => {
                state.view(selected_theme).map(Message::FindChannelsMessage)
            }
        }
    }
}

const NAVBAR_WIDTH: u16 = 55;
const PADDING_V: u16 = 6;
const PADDING_H: u16 = 6;
const ICON_SIZE: u16 = 26;
