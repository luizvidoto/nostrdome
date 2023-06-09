#![allow(unused_variables)]
use iced::Subscription;

use crate::{
    net::{BackEndConnection, BackendEvent},
    style,
    widget::Element,
};

use super::RouterCommand;

pub trait Route: Sized {
    type Message: std::fmt::Debug + Send;

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        RouterCommand::new()
    }

    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        RouterCommand::new()
    }

    fn view(&self, selected_theme: Option<style::Theme>) -> Element<'_, Self::Message>;
}
