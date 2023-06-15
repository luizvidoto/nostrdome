use crate::components::{card, common_scrollable};
use crate::db::{DbRelay, DbRelayResponse};
use crate::net::BackEndConnection;
use crate::style;
use crate::widget::Element;
use iced::alignment;
use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Command, Length};
use iced_aw::Modal;
use std::fmt::Debug;

use super::ModalView;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
}

pub struct RelaysConfirmation<M: Clone + Debug> {
    responses: Vec<DbRelayResponse>,
    all_relays: Vec<DbRelay>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Clone + Debug> RelaysConfirmation<M> {
    pub fn new(responses: &[DbRelayResponse], all_relays: &[DbRelay]) -> Self {
        Self {
            responses: responses.to_vec(),
            all_relays: all_relays.to_vec(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<M: Clone + Debug + 'static + Send> ModalView for RelaysConfirmation<M> {
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
        }
        Ok((command, false))
    }

    fn view<'a>(
        &'a self,
        underlay: impl Into<Element<'a, Self::UnderlayMessage>>,
    ) -> Element<'a, Self::Message> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);
        Modal::new(true, underlay_component, move || {
            let title_txt = format!(
                "Relays Confirmation {}/{}",
                self.responses.len(),
                self.all_relays.len()
            );
            let title = container(text(title_txt).size(22)).center_x();

            let col = column![].spacing(10);
            let content = self
                .responses
                .iter()
                .fold(col, |col, response| col.push(make_response_row(response)));

            let card_body = common_scrollable(
                container(column![title, content].spacing(15))
                    .center_x()
                    .padding(20),
            );

            let card_footer =
                row![
                    button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(CMessage::CloseModal),
                ]
                .spacing(10);

            card(card_body, card_footer).max_width(MODAL_WIDTH).into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }
}

fn make_response_row<'a, M: 'a>(response: &DbRelayResponse) -> Element<'a, M> {
    let (_, error_msg) = response.status.to_bool();
    let url_txt = text(&response.relay_url);

    let status_txt: Element<_> = if let Some(error_msg) = &error_msg {
        tooltip(text("Failed"), error_msg, tooltip::Position::Top)
            .style(style::Container::TooltipBg)
            .into()
    } else {
        text("Ok").into()
    };

    row![url_txt, Space::with_width(Length::Fill), status_txt,]
        .spacing(5)
        .padding(5)
        .into()
}

const MODAL_WIDTH: f32 = 300.0;
