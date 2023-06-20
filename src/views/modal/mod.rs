#![allow(unused_variables)]

pub(crate) mod basic_contact;
pub(crate) mod import_contact_list;
pub(crate) mod relay_basic;
pub(crate) mod relay_document;
pub(crate) mod relays_confirmation;

pub(crate) use basic_contact::ContactDetails;
pub(crate) use import_contact_list::ImportContactList;
pub(crate) use relay_basic::RelayBasic;
pub(crate) use relay_document::RelayDocState;
pub(crate) use relays_confirmation::RelaysConfirmation;

use crate::{
    error::BackendClosed,
    net::{BackEndConnection, BackendEvent},
    widget::Element,
};
use iced::Command;
use std::fmt::Debug;

pub trait ModalView {
    type UnderlayMessage: Debug + Clone + Send;
    type Message: Debug + Clone + Send;

    /// Returns true to close modal
    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> Result<(Command<Self::Message>, bool), BackendClosed>;

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        Ok(())
    }

    fn view<'a>(
        &'a self,
        underlay: impl Into<Element<'a, Self::UnderlayMessage>>,
    ) -> Element<'a, Self::Message>;
}
