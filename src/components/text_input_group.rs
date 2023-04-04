use iced::{
    widget::{column, text, text_input},
    Element,
};

pub fn text_input_group<'a, Message: Clone + 'a>(
    label_str: &str,
    placeholder: &str,
    value: &str,
    on_change: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let label = text(label_str);
    let txt_input = text_input(placeholder, value, on_change);
    column![label, txt_input].into()
}
