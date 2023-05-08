use iced::widget::scrollable;

use crate::widget::{Element, Scrollable};

pub fn common_scrollable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    scrollable(content.into()).vertical_scroll(
        scrollable::Properties::new()
            .width(6.0)
            .margin(0.0)
            .scroller_width(6.0),
    )
}
