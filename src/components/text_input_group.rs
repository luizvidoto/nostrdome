use iced::widget::{column, container, row, text, text_input, tooltip};

use crate::{style, widget::Element};

pub struct TextInputGroup<'a, Message: Clone + 'a> {
    label_str: &'a str,
    placeholder: &'a str,
    value: &'a str,
    tooltip_str: Option<String>,
    on_change: Box<dyn Fn(String) -> Message + 'a>,
    on_submit: Option<Message>,
    is_invalid: bool,
    invalid_message: String,
    is_disabled: bool,
}

impl<'a, Message: Clone + 'a> TextInputGroup<'a, Message> {
    pub fn new(
        label_str: &'a str,
        value: &'a str,
        on_change: impl Fn(String) -> Message + 'a,
    ) -> Self {
        Self {
            label_str,
            placeholder: "",
            value,
            tooltip_str: None,
            on_change: Box::new(on_change),
            on_submit: None,
            is_invalid: false,
            invalid_message: String::from(""),
            is_disabled: false,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip_str = Some(tooltip.to_owned());
        self
    }

    pub fn on_submit(mut self, on_submit: Message) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    pub fn invalid(mut self, invalid_message: &str) -> Self {
        self.is_invalid = true;
        self.invalid_message = invalid_message.to_owned();
        self
    }

    pub(crate) fn disabled(mut self) -> Self {
        self.is_disabled = true;
        self
    }

    pub fn build(self) -> Element<'a, Message> {
        text_input_group(
            self.label_str,
            self.placeholder,
            self.value,
            self.tooltip_str,
            self.on_change,
            self.on_submit,
            self.is_invalid,
            &self.invalid_message,
            self.is_disabled,
        )
    }
}

fn text_input_group<'a, Message: Clone + 'a>(
    label_str: &str,
    placeholder: &str,
    value: &str,
    tooltip_str: Option<String>,
    on_change: impl Fn(String) -> Message + 'a,
    on_submit: Option<Message>,
    is_invalid: bool,
    invalid_message: &str,
    is_disabled: bool,
) -> Element<'a, Message> {
    let text_input_style = if is_invalid {
        style::TextInput::Invalid
    } else {
        style::TextInput::ChatSearch
    };

    let label_style = if is_invalid {
        style::Text::Danger
    } else {
        style::Text::Default
    };

    let label = text(label_str).style(label_style);
    let tooltip: Element<_> = if let Some(tooltip_str) = tooltip_str {
        let tooltip_icon = container(text("?").size(15))
            .style(style::Container::TooltipIcon)
            .padding([2, 4]);
        tooltip(tooltip_icon, tooltip_str, tooltip::Position::Top)
            .style(style::Container::TooltipBg)
            .into()
    } else {
        text("").into()
    };
    let label_row = row![label, tooltip].spacing(4);

    let mut txt_input = text_input(placeholder, value).style(text_input_style);
    if !is_disabled {
        txt_input = txt_input.on_input(on_change);
        if let Some(on_submit) = on_submit {
            txt_input = txt_input.on_submit(on_submit);
        }
    }

    let invalid_message_text = text(invalid_message).size(16).style(style::Text::Danger);

    column![label_row, txt_input, invalid_message_text].into()
}
