use iced::widget::{button, tooltip};

use crate::{icon::copy_icon, style, widget::Element};

pub fn copy_btn<'a, M: 'a + Clone>(text: &str, message: M) -> Element<'a, M> {
    tooltip(
        button(copy_icon())
            .on_press(message)
            .width(30)
            .style(style::Button::Primary),
        text,
        tooltip::Position::Top,
    )
    .style(style::Container::TooltipBg)
    .into()
}
