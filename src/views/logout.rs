use crate::components::inform_card;
use crate::net::{BackEndConnection, BackendEvent};
use crate::style;
use crate::widget::Element;

use super::route::Route;
use super::{RouterCommand, RouterMessage};

#[derive(Debug, Clone)]
pub enum Message {}
pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }
}
impl Route for State {
    type Message = Message;

    fn backend_event(
        &mut self,
        event: BackendEvent,
        _conn: &mut BackEndConnection,
    ) -> RouterCommand<Self::Message> {
        let mut command = RouterCommand::new();
        match event {
            BackendEvent::LogoutSuccess => command.change_route(RouterMessage::GoToLogin),
            _ => (),
        }
        command
    }

    fn view(&self, _selected_theme: Option<style::Theme>) -> Element<'_, Self::Message> {
        inform_card("Logging out", "Please wait...").into()
    }
}
