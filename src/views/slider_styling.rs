use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Command, Length, Subscription};
use iced_native::widget::slider;

use crate::components::text::title;
use crate::net::{BackEndConnection, BackendEvent};
use crate::views::RouterCommand;
use crate::{
    icon::{regular_bell_icon, search_icon},
    style,
    widget::Element,
};

#[derive(Debug, Clone)]
pub enum Message {
    ChangedPaddingV(u16),
    ChangedPaddingH(u16),
    ChangedIconSize(u16),
    ChangedNavbarWidth(u16),
}
pub struct State {}
impl State {
    pub fn new(_conn: &mut BackEndConnection) -> Self {
        Self {}
    }
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
    pub(crate) fn update(
        &mut self,
        _message: Message,
        _conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let commands = RouterCommand::new();

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
    pub fn view(
        &self,
        padding_v: u16,
        padding_h: u16,
        navbar_width: u16,
        icon_size: u16,
    ) -> Element<Message> {
        let title = title("Menu Styling");

        let padding_v_control = column![
            text("Padding Vertical"),
            row![
                slider(0..=10, padding_v, Message::ChangedPaddingV),
                text(&padding_v)
            ]
            .spacing(2),
        ]
        .spacing(2);
        let padding_h_control = column![
            text("Padding Horizontal"),
            row![
                slider(0..=10, padding_h, Message::ChangedPaddingH),
                text(&padding_h)
            ]
            .spacing(2),
        ]
        .spacing(2);
        let navbar_control = column![
            text("Navbar Width"),
            row![
                slider(10..=100, navbar_width, Message::ChangedNavbarWidth),
                text(&navbar_width)
            ]
            .spacing(2),
        ]
        .spacing(2);
        let icon_size_control = column![
            text("Icon Size"),
            row![
                slider(10..=40, icon_size, Message::ChangedIconSize),
                text(&icon_size)
            ]
            .spacing(2),
        ]
        .spacing(2);

        let controls = container(
            column![
                padding_v_control,
                padding_h_control,
                navbar_control,
                icon_size_control
            ]
            .spacing(10),
        )
        .width(500)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .padding(20)
        .style(style::Container::Bordered);

        column![title, controls].into()
    }
}
