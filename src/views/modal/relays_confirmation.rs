use crate::components::common_scrollable;
use crate::db::{DbRelay, DbRelayResponse};
use crate::net::BackEndConnection;
use crate::style;
use crate::widget::Element;
use iced::alignment;
use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Command, Length};
use iced_aw::Modal;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
}

pub struct RelaysConfirmation {
    responses: Vec<DbRelayResponse>,
    all_relays: Vec<DbRelay>,
}
impl RelaysConfirmation {
    pub fn new(responses: &[DbRelayResponse], all_relays: &[DbRelay]) -> Self {
        Self {
            responses: responses.to_vec(),
            all_relays: all_relays.to_vec(),
        }
    }
    pub fn update<'a, M: 'a + Clone + Debug>(
        &'a mut self,
        message: CMessage<M>,
        _conn: &mut BackEndConnection,
    ) -> (Command<CMessage<M>>, bool) {
        let command = Command::none();
        match message {
            CMessage::UnderlayMessage(_) => (),
            CMessage::CloseModal => return (command, true),
        }
        (command, false)
    }
    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);
        Modal::new(true, underlay_component, move || {
            let title_txt = format!(
                "Relays Confirmation {}/{}",
                self.responses.len(),
                self.all_relays.len()
            );
            let title = container(text(title_txt).size(22)).center_x();
            let header = container(title)
                .width(Length::Fill)
                .height(HEADER_HEIGHT)
                .align_y(alignment::Vertical::Top)
                .center_x();
            let col = column![].spacing(10);
            let content = self
                .responses
                .iter()
                .fold(col, |col, response| col.push(make_response_row(response)));

            let card_body: Element<_> = container(common_scrollable(content))
                .width(Length::Fill)
                .center_x()
                .center_y()
                .into();
            let card_footer = container(
                row![
                    button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(CMessage::CloseModal),
                ]
                .spacing(10),
            )
            .align_y(alignment::Vertical::Bottom)
            .center_x()
            .height(FOOTER_HEIGHT)
            .width(Length::Fill);

            container(column![header, card_body, card_footer].spacing(20))
                .padding(10)
                .max_width(MODAL_WIDTH)
                .height(Length::Shrink)
                .width(Length::Shrink)
                .style(style::Container::Modal)
                .into()
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
const HEADER_HEIGHT: f32 = 60.0;
const FOOTER_HEIGHT: f32 = 60.0;
