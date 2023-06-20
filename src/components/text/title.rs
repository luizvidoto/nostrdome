use iced::{
    widget::{container, text},
    Length,
};

use crate::widget::Container;

pub fn title<'a, Message: 'a>(title: impl Into<String>) -> Container<'a, Message> {
    container(text(title.into()).size(30))
        .width(Length::Fill)
        .padding([5, 0])
}
