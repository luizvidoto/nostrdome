use iced::widget::{column, container};
use iced::Length;

use crate::{
    style,
    widget::{Container, Element},
};

use super::text::title;

pub fn card<'a, M: 'a>(
    card_body: impl Into<Element<'a, M>>,
    card_footer: impl Into<Element<'a, M>>,
) -> Container<'a, M> {
    let card_body = container(card_body).max_height(CARD_MAX_HEIGHT - CARD_FOOTER_HEIGHT);
    let card_footer = container(card_footer)
        .center_y()
        .style(style::Container::CardFoot)
        .padding(5)
        .height(CARD_FOOTER_HEIGHT)
        .width(Length::Fill);

    container(column![card_body, card_footer])
        .max_height(CARD_MAX_HEIGHT)
        .style(style::Container::CardBody)
}

const CARD_FOOTER_HEIGHT: u16 = 50;
const CARD_MAX_HEIGHT: u16 = 400;

pub fn inform_card<'a, Message: 'a>(
    title_text: &str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = column![title(title_text).width(Length::Shrink), content.into()].spacing(10);
    let inner = container(content)
        .padding(30)
        .style(style::Container::Frame);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(style::Container::Background)
        .into()
}
