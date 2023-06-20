use crate::components::inform_card;
use crate::error::BackendClosed;
use crate::net::{BackEndConnection, BackendEvent};
use crate::style;
use crate::widget::Element;

use super::route::Route;
use super::{GoToView, RouterCommand};

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
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut command = RouterCommand::new();

        if let BackendEvent::LogoutSuccess = event {
            command.change_route(GoToView::Login);
        }

        Ok(command)
    }

    fn view(&self, _selected_theme: Option<style::Theme>) -> Element<'_, Self::Message> {
        inform_card("Logging out", "Please wait...")
    }
}
