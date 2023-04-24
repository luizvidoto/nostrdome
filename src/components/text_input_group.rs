use iced::widget::{column, container, row, text, text_input, tooltip};

use crate::{style, widget::Element};

pub fn text_input_group<'a, Message: Clone + 'a>(
    label_str: &str,
    placeholder: &str,
    value: &str,
    tooltip_str: Option<String>,
    on_change: impl Fn(String) -> Message + 'a,
    on_submit: Option<Message>,
) -> Element<'a, Message> {
    let label = text(label_str);
    let tooltip: Element<_> = if let Some(tooltip_str) = tooltip_str {
        let tooltip_icon = container(text("?").size(15))
            .style(style::Container::TooltipContainer)
            .padding([2, 4]);
        tooltip(tooltip_icon, tooltip_str, tooltip::Position::Top).into()
    } else {
        text("").into()
    };
    let label_row = row![label, tooltip].spacing(4);
    let mut txt_input = text_input(placeholder, value).on_input(on_change);
    if let Some(on_submit) = on_submit {
        txt_input = txt_input.on_submit(on_submit);
    }

    column![label_row, txt_input].into()
}
