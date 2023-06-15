use iced::{
    alignment,
    widget::{button, column, container, row, text, Space},
    Alignment, Length,
};
use nostr::{prelude::FromSkStr, Keys};

use crate::{
    components::{text::title, text_input_group::TextInputGroup},
    error::BackendClosed,
    net::{BackEndConnection, BackendEvent, ToBackend},
    style,
    widget::Element,
};

use super::{route::Route, RouterCommand, RouterMessage};

#[derive(Debug, Clone)]
pub struct BasicProfile {
    pub name: String,
    pub about: String,
    pub profile_picture: String,
}
impl BasicProfile {
    pub fn new(name: String, about: String, profile_picture: String) -> Self {
        Self {
            name,
            about,
            profile_picture,
        }
    }
}
impl From<BasicProfile> for nostr::Metadata {
    fn from(profile: BasicProfile) -> Self {
        Self {
            name: Some(profile.name),
            about: Some(profile.about),
            picture: Some(profile.profile_picture),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SecretKeyInputChange(String),
    SubmitPress(String),
    ToCreateAccount,
    ToImportAccount,
    ToChooseAccount,
    CreateAccountSubmit(BasicProfile),
    NameInputChange(String),
    AboutInputChange(String),
    ProfilePictureInputChange(String),
}

#[allow(dead_code)]
pub enum State {
    ChooseAccount,
    CreateAccount {
        name: String,
        about: String,
        profile_picture: String,
        is_profile_pic_invalid: bool,
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
            // secret_key_input: "".into(),
            is_invalid: false,
        }
    }
    pub fn create_account() -> Self {
        Self::CreateAccount {
            name: "".into(),
            about: "".into(),
            profile_picture: "".into(),
            is_profile_pic_invalid: false,
        }
    }
}
impl Route for State {
    type Message = Message;

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut command = RouterCommand::new();

        match event {
            BackendEvent::LoginSuccess => {
                conn.send(ToBackend::QueryFirstLogin)?;
            }
            BackendEvent::FinishedPreparing => {
                command.change_route(RouterMessage::GoToChat);
            }
            BackendEvent::FirstLoginSuccess => {
                command.change_route(RouterMessage::GoToWelcome);
            }
            BackendEvent::CreateAccountSuccess => {
                command.change_route(RouterMessage::GoToWelcome);
            }
            _ => (),
        }

        Ok(command)
    }

    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let command = RouterCommand::new();

        match self {
            State::ChooseAccount => match message {
                Message::ToCreateAccount => *self = Self::create_account(),
                Message::ToImportAccount => *self = Self::import_account(),
                _ => (),
            },
            State::CreateAccount {
                name: name_input,
                about: about_input,
                profile_picture: profile_picture_input,
                is_profile_pic_invalid,
            } => match message {
                Message::NameInputChange(text) => *name_input = text,
                Message::AboutInputChange(text) => *about_input = text,
                Message::ProfilePictureInputChange(text) => {
                    *profile_picture_input = text;
                    *is_profile_pic_invalid = false;
                }
                Message::ToChooseAccount => *self = Self::new(),
                Message::CreateAccountSubmit(profile) => {
                    conn.send(ToBackend::CreateAccount(profile.into()))?;
                }
                _ => (),
            },
            State::ImportAccount {
                secret_key_input,
                is_invalid,
            } => match message {
                Message::SecretKeyInputChange(secret_key) => {
                    *secret_key_input = secret_key;
                    *is_invalid = false;
                }
                Message::SubmitPress(secret_key) => match Keys::from_sk_str(&secret_key) {
                    Ok(keys) => {
                        conn.send(ToBackend::LoginWithSK(keys))?;
                    }
                    Err(e) => {
                        tracing::error!("Invalid secret key: {}", e);
                        *is_invalid = true;
                    }
                },
                Message::ToChooseAccount => *self = Self::new(),
                _ => (),
            },
        }

        Ok(command)
    }

    fn view(&self, _selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        let content: Element<_> = match self {
            State::ChooseAccount => {
                let page_title = title("Sign In").center_x();
                let create_acc_btn = big_button("Create Nostr Account", Message::ToCreateAccount);
                let import_acc_btn = big_button("Import With Keys", Message::ToImportAccount);

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
                name,
                about,
                profile_picture,
                is_profile_pic_invalid,
            } => {
                let name_input = TextInputGroup::new("Name", name, Message::NameInputChange);
                let about_input = TextInputGroup::new("About", about, Message::AboutInputChange);
                let mut profile_pic_input = TextInputGroup::new(
                    "Profile Picture",
                    profile_picture,
                    Message::ProfilePictureInputChange,
                );

                let back_btn = button("Back")
                    .padding(10)
                    .style(style::Button::Invisible)
                    .on_press(Message::ToChooseAccount);
                let mut submit_btn = button("Submit").padding(10).style(style::Button::Primary);

                if *is_profile_pic_invalid {
                    profile_pic_input = profile_pic_input.invalid("Invalid profile picture URL");
                } else {
                    submit_btn = submit_btn.on_press(Message::CreateAccountSubmit(
                        BasicProfile::new(name.clone(), about.clone(), profile_picture.clone()),
                    ));
                }

                let buttons = row![back_btn, Space::with_width(Length::Fill), submit_btn]
                    .align_items(Alignment::Center)
                    .spacing(10);
                column![
                    title("Create Nostr Account"),
                    name_input.build(),
                    about_input.build(),
                    profile_pic_input.build(),
                    buttons
                ]
                .spacing(20)
                .into()
            }
            State::ImportAccount {
                secret_key_input,
                is_invalid,
            } => {
                let mut secret_input = TextInputGroup::new(
                    "Secret Key",
                    secret_key_input,
                    Message::SecretKeyInputChange,
                )
                .placeholder("a4s6d84as6d4a...")
                .on_submit(Message::SubmitPress(secret_key_input.clone()));

                if *is_invalid {
                    secret_input = secret_input.invalid("Invalid Secret Key");
                }

                let back_btn = button("Back")
                    .style(style::Button::Invisible)
                    .padding(10)
                    .on_press(Message::ToChooseAccount);
                let submit_btn = button("Submit")
                    .padding(10)
                    .style(style::Button::Primary)
                    .on_press(Message::SubmitPress(secret_key_input.clone()));
                let buttons = row![back_btn, Space::with_width(Length::Fill), submit_btn]
                    .align_items(Alignment::Center)
                    .spacing(10);
                column![title("Login"), secret_input.build(), buttons]
                    .spacing(20)
                    .into()
            }
        };

        let form = container(content)
            .width(400.0)
            .padding(30)
            .style(style::Container::Frame);

        container(form)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(style::Container::Background)
            .into()
    }
}

fn big_button(title: &str, message: Message) -> Element<'static, Message> {
    button(
        container(
            text(title)
                .horizontal_alignment(alignment::Horizontal::Center)
                .vertical_alignment(alignment::Vertical::Center),
        )
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_press(message)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(style::Button::Primary)
    .into()
}
