use iced::widget::{button, column, container, row, text, Space};
use iced::{alignment, Command, Length, Subscription};

use crate::db::DbContact;
use crate::icon::{server_icon, settings_icon};
use crate::net::{BackEndConnection, BackendEvent};

use crate::widget::{Button, Rule, Text};
use crate::{
    icon::{home_icon, search_icon},
    style,
    widget::Element,
};

use super::{chat, RouterCommand, RouterMessage};

#[derive(Debug, Clone)]
pub enum Message {
    DMsPressed,
    FindChannelsPressed,
    SettingsPressed,
    DmsMessage(chat::Message),
    FindChannelsMessage(find_channel::Message),
}
pub struct State {
    active_view: ViewState,
}
impl State {
    pub(crate) fn chat(conn: &mut BackEndConnection) -> Self {
        Self {
            active_view: ViewState::DMs {
                state: chat::State::new(conn),
            },
        }
    }
    pub(crate) fn chat_to(db_contact: DbContact, conn: &mut BackEndConnection) -> Self {
        Self {
            active_view: ViewState::DMs {
                state: chat::State::chat_to(db_contact, conn),
            },
        }
    }
    pub(crate) fn find_channels(conn: &mut BackEndConnection) -> State {
        Self {
            active_view: ViewState::FindChannel {
                state: find_channel::State::new(conn),
            },
        }
    }
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        self.active_view.subscription()
    }
    pub(crate) fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        self.active_view.backend_event(event.clone(), conn)
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
                    state: find_channel::State::new(conn),
                }
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

        let active_view = container(self.active_view.view())
            .width(Length::Fill)
            .height(Length::Fill);

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
    FindChannel { state: find_channel::State },
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

mod find_channel {
    use iced::widget::{button, column, container, row, text, Space};
    use iced::{Alignment, Command, Length, Subscription};
    use iced_native::widget::text_input;

    use crate::components::common_scrollable;
    use crate::components::text::title;
    use crate::net::{BackEndConnection, BackendEvent};
    use crate::views::RouterCommand;
    use crate::widget::Rule;
    use crate::{
        icon::{regular_bell_icon, search_icon},
        style,
        widget::Element,
    };

    #[derive(Debug, Clone)]
    pub enum Message {
        SearchInputChanged(String),
    }
    pub struct State {
        search_input_value: String,
    }
    impl State {
        pub fn new(_conn: &mut BackEndConnection) -> Self {
            Self {
                search_input_value: String::new(),
            }
        }
        pub(crate) fn subscription(&self) -> Subscription<Message> {
            Subscription::none()
        }
        pub(crate) fn update(
            &mut self,
            message: Message,
            _conn: &mut BackEndConnection,
        ) -> RouterCommand<Message> {
            let commands = RouterCommand::new();
            match message {
                Message::SearchInputChanged(text) => {
                    self.search_input_value = text;
                }
            }
            commands
        }
        pub(crate) fn backend_event(
            &mut self,
            event: BackendEvent,
            _conn: &mut BackEndConnection,
        ) -> RouterCommand<Message> {
            let commands = RouterCommand::new();

            commands
        }
        pub fn view(&self) -> Element<Message> {
            let title = title("Find Channels");
            let search_input =
                text_input("Channel name, type, about, etc..", &self.search_input_value)
                    .on_input(Message::SearchInputChanged)
                    .size(30);
            let results_container = container(column![
                make_results(),
                make_results(),
                make_results(),
                make_results(),
            ]);

            common_scrollable(
                container(column![title, search_input, results_container]).padding([20, 0, 0, 20]),
            )
            .into()
        }
    }

    fn make_results<'a, M: 'a>() -> Element<'a, M> {
        let image_container = container(text("Image 1"))
            .width(IMAGE_WIDTH)
            .height(IMAGE_HEIGHT)
            .center_x()
            .center_y()
            .style(style::Container::Bordered);

        let channel_about = container(
            column![
                text("Channel Name").size(22),
                text("Channel about").size(18),
                Space::with_height(Length::Fill),
                text("Members").size(14)
            ]
            .spacing(5),
        )
        .center_y()
        .height(IMAGE_HEIGHT)
        .width(Length::Fill);

        container(
            column![
                row![image_container, channel_about].spacing(20),
                Rule::horizontal(10)
            ]
            .spacing(10),
        )
        .padding(10)
        .max_width(MAX_WIDTH_RESULT)
        .into()
    }
    const MAX_WIDTH_RESULT: u16 = 600;
    const IMAGE_WIDTH: u16 = 220;
    const IMAGE_HEIGHT: u16 = 120;
}

const NAVBAR_WIDTH: u16 = 55;
const PADDING_V: u16 = 6;
const PADDING_H: u16 = 6;
const ICON_SIZE: u16 = 26;
