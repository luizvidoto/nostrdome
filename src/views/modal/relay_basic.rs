use super::ModalView;
use crate::components::card;
use crate::components::text_input_group::TextInputGroup;
use crate::net::{BackEndConnection, ToBackend};
use crate::style;
use crate::widget::Element;
use iced::alignment::Horizontal;
use iced::widget::{button, container, row, text};
use iced::{Command, Length};
use iced_aw::Modal;
use std::fmt::Debug;
use url::Url;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
    AddressInputChange(String),
    OkButtonPressed,
}

pub struct RelayBasic<M: Clone + Debug> {
    address_input: String,
    is_invalid: bool,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Clone + Debug> RelayBasic<M> {
    pub fn new() -> Self {
        Self {
            is_invalid: false,
            address_input: "".into(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<M: Clone + Debug + 'static + Send> ModalView for RelayBasic<M> {
    type UnderlayMessage = M;
    type Message = CMessage<M>;

    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> Result<(Command<Self::Message>, bool), crate::error::BackendClosed> {
        let command = Command::none();
        match message {
            CMessage::UnderlayMessage(_) => (),
            CMessage::CloseModal => return Ok((command, true)),
            CMessage::AddressInputChange(text) => {
                self.is_invalid = false;
                self.address_input = text;
            }
            CMessage::OkButtonPressed => match Url::try_from(self.address_input.as_str()) {
                Ok(url) => {
                    self.is_invalid = false;
                    self.address_input = "".into();
                    conn.send(ToBackend::AddRelay(url))?;
                    return Ok((command, true));
                }
                Err(e) => {
                    tracing::error!("{}", e);
                    self.is_invalid = true;
                }
            },
        }
        Ok((command, false))
    }

    fn view<'a>(
        &'a self,
        underlay: impl Into<Element<'a, Self::UnderlayMessage>>,
    ) -> Element<'a, Self::Message> {
        let underlay_component: Element<_> = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, move || {
            let mut add_relay_input = TextInputGroup::new(
                "Relay Address",
                &self.address_input,
                CMessage::AddressInputChange,
            )
            .placeholder("wss://my-relay.com")
            .on_submit(CMessage::OkButtonPressed);

            if self.is_invalid {
                add_relay_input = add_relay_input.invalid("Relay address is invalid");
            }

            let card_body = container(add_relay_input.build()).padding(20);
            let card_footer = row![
                button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                    .style(style::Button::Bordered)
                    .width(Length::Fill)
                    .on_press(CMessage::CloseModal),
                button(text("Ok").horizontal_alignment(Horizontal::Center),)
                    .style(style::Button::Primary)
                    .width(Length::Fill)
                    .on_press(CMessage::OkButtonPressed)
            ]
            .spacing(10);

            card(card_body, card_footer).max_width(MODAL_WIDTH).into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }
}

const MODAL_WIDTH: f32 = 300.0;
