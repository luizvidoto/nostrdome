use iced::{
    alignment,
    widget::{button, column, container, row, text, Space},
    Length,
};

use crate::{
    components::{text::title, text_input_group::text_input_group},
    style,
    widget::Element,
};

#[derive(Debug, Clone)]
pub enum Message {
    SecretKeyInputChange(String),
    SubmitPress(String),
    ToCreateAccount,
    ToImportAccount,
    ToChooseAccount,
    CreateAccountSubmit,
    NameInputChange(String),
    AboutInputChange(String),
    ProfilePictureInputChange(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum State {
    ChooseAccount,
    CreateAccount {
        name_input: String,
        about_input: String,
        profile_picture_input: String,
    },
    ImportAccount {
        secret_key_input: String,
        is_invalid: bool,
    },
}
impl State {
    pub fn new() -> Self {
        Self::ChooseAccount
    }
    pub fn import_account() -> Self {
        Self::ImportAccount {
            secret_key_input: "4510459b74db68371be462f19ef4f7ef1e6c5a95b1d83a7adf00987c51ac56fe"
                .into(),
            is_invalid: false,
        }
    }
    pub fn create_account() -> Self {
        Self::CreateAccount {
            name_input: "".into(),
            about_input: "".into(),
            profile_picture_input: "".into(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match self {
            State::ChooseAccount => match message {
                Message::ToCreateAccount => *self = Self::create_account(),
                Message::ToImportAccount => *self = Self::import_account(),
                _ => (),
            },
            State::CreateAccount {
                name_input,
                about_input,
                profile_picture_input,
            } => match message {
                Message::NameInputChange(text) => *name_input = text,
                Message::AboutInputChange(text) => *about_input = text,
                Message::ProfilePictureInputChange(text) => *profile_picture_input = text,
                Message::ToChooseAccount => *self = Self::new(),
                Message::CreateAccountSubmit => {
                    println!("name: {}", name_input);
                }
                _ => (),
            },
            State::ImportAccount {
                secret_key_input,
                is_invalid,
            } => match message {
                Message::SecretKeyInputChange(secret_key) => *secret_key_input = secret_key,
                Message::SubmitPress(_) => (),
                Message::ToChooseAccount => *self = Self::new(),
                _ => (),
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        let content: Element<_> = match self {
            State::ChooseAccount => {
                let page_title = title("Sign In").center_x();
                let create_acc_btn = button(
                    container(
                        text("Create Nostr Account")
                            .horizontal_alignment(alignment::Horizontal::Center)
                            .vertical_alignment(alignment::Vertical::Center),
                    )
                    .center_x()
                    .center_y()
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .on_press(Message::ToCreateAccount)
                .width(Length::Fill)
                .height(Length::Fill);

                let import_acc_btn = button(
                    container(
                        text("Import With Keys")
                            .horizontal_alignment(alignment::Horizontal::Center)
                            .vertical_alignment(alignment::Vertical::Center),
                    )
                    .center_x()
                    .center_y()
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .on_press(Message::ToImportAccount)
                .width(Length::Fill)
                .height(Length::Fill);

                let buttons = row![create_acc_btn, import_acc_btn]
                    .height(100.0)
                    .spacing(20)
                    .width(Length::Fill);
                column![page_title, buttons]
                    .spacing(20)
                    .width(Length::Fill)
                    .into()
            }
            State::CreateAccount {
                name_input,
                about_input,
                profile_picture_input,
            } => {
                let name_input =
                    text_input_group("Name", "", name_input, None, Message::NameInputChange, None);
                let about_input = text_input_group(
                    "About",
                    "",
                    about_input,
                    None,
                    Message::AboutInputChange,
                    None,
                );
                let profile_pic_input = text_input_group(
                    "Profile Picture",
                    "",
                    profile_picture_input,
                    None,
                    Message::ProfilePictureInputChange,
                    None,
                );
                let back_btn = button("Back")
                    .style(style::Button::Invisible)
                    .on_press(Message::ToChooseAccount);
                let submit_btn = button("Submit").on_press(Message::CreateAccountSubmit);
                let buttons =
                    row![back_btn, Space::with_width(Length::Fill), submit_btn].spacing(10);
                column![
                    title("Create Nostr Account"),
                    name_input,
                    about_input,
                    profile_pic_input,
                    buttons
                ]
                .spacing(20)
                .into()
            }
            State::ImportAccount {
                secret_key_input,
                is_invalid,
            } => {
                let secret_input = text_input_group(
                    "Secret Key",
                    "a4s6d84as6d4a...",
                    secret_key_input,
                    None,
                    Message::SecretKeyInputChange,
                    Some(Message::SubmitPress(secret_key_input.clone())),
                );
                let back_btn = button("Back")
                    .style(style::Button::Invisible)
                    .on_press(Message::ToChooseAccount);
                let submit_btn =
                    button("Submit").on_press(Message::SubmitPress(secret_key_input.clone()));
                let buttons =
                    row![back_btn, Space::with_width(Length::Fill), submit_btn].spacing(10);
                column![title("Login"), secret_input, buttons]
                    .spacing(20)
                    .into()
            }
        };

        let form = container(content)
            .width(400.0)
            .padding(30)
            .style(style::Container::ContactList);

        container(form)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(style::Container::ChatContainer)
            .into()
    }
}
