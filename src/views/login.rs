use iced::{
    widget::{button, column, container},
    Element,
};

use crate::components::{text::title, text_input_group::text_input_group};

#[derive(Debug, Clone)]
pub enum Message {
    SecretKeyInputChange(String),
    SubmitPress(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct State {
    secret_key_input: String,
    is_invalid: bool,
}
impl State {
    pub fn new() -> Self {
        Self {
            secret_key_input: "".into(),
            is_invalid: false,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SecretKeyInputChange(secret_key) => self.secret_key_input = secret_key,
            // Message::SecretKeyInputChange(_secret_key) => (),
            Message::SubmitPress(_) => (),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let secret_input = text_input_group(
            "Secret Key",
            "a4s6d84as6d4a...",
            &self.secret_key_input,
            None,
            Message::SecretKeyInputChange,
        );
        let submit_btn =
            button("Submit").on_press(Message::SubmitPress(self.secret_key_input.clone()));
        let page_title = title("Login");
        let content = column![page_title, secret_input, submit_btn].spacing(10);

        container(content).width(300.0).into()
    }
}
