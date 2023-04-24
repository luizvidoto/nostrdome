use iced::widget::{container, text};

use crate::widget::Element;

pub fn title<'a, Message: 'a>(title: impl Into<String>) -> Element<'a, Message> {
    container(text(title.into()).size(30))
        .padding([5, 0])
        .into()
}
