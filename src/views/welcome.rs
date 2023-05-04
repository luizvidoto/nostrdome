use iced::widget::{button, column, container, image, row, text};
use iced::{Alignment, Length};

use crate::consts::{CONTACTS_IMAGE, RELAYS_IMAGE, WELCOME_IMAGE};
use crate::icon::{regular_circle_icon, solid_circle_icon};
use crate::style;
use crate::{components::text::title, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    ToNextStep,
    ToPreviousStep,
    ToApp,
    AddRelay(String),
    ImportContactsRelays,
    ImportContactsFile,
}
pub struct State {
    pub step: usize,
    pub relays_added: Vec<String>,
    pub loading_client: bool,
}
impl State {
    pub fn new() -> Self {
        Self {
            loading_client: false,
            step: 0,
            relays_added: vec![],
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToNextStep => self.step += 1,
            Message::ToPreviousStep => self.step -= 1,
            Message::ToApp => {
                self.loading_client = true;
            }
            Message::AddRelay(relay_url) => {
                self.relays_added.push(relay_url);
            }
            Message::ImportContactsRelays => {
                tracing::info!("Importing contacts from relays");
            }
            Message::ImportContactsFile => {
                tracing::info!("Importing contacts from a file");
            }
        }
    }
    pub fn view(&self) -> Element<Message> {
        let title_1 = "NostrDome";
        let text_1a = "Secure, encrypted chats on the NOSTR network";
        let title_2 = "Relays Setup";
        let text_2 = "Add relays to connect";
        let title_3 = "Contacts";
        let text_3a = "Import from relays";
        let text_3b = "Import from file";

        let relay_list = vec![
            "wss://relay.plebstr.com",
            "wss://nostr.wine",
            "wss://relay.nostr.info",
            "wss://relay.snort.social",
            "ws://192.168.15.151:8080",
        ]
        .iter()
        .fold(column![].spacing(5), |column, relay| {
            let relay_btn = if self.relays_added.contains(&relay.to_string()) {
                button("Ok")
            } else {
                button("Add").on_press(Message::AddRelay(relay.to_string()))
            };
            column.push(
                container(
                    row![text(relay).size(20).width(Length::Fill), relay_btn]
                        .align_items(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Shrink),
            )
        });
        let relays_sugestions = container(relay_list)
            .padding(20)
            .style(style::Container::Bordered)
            .width(Length::Fill)
            .height(Length::Fill);

        let welcome_image = image::Image::new(image::Handle::from_memory(WELCOME_IMAGE));
        let relays_image = image::Image::new(image::Handle::from_memory(RELAYS_IMAGE));
        let contacts_image = image::Image::new(image::Handle::from_memory(CONTACTS_IMAGE));

        let last_step_buttons: Element<_> = if self.loading_client {
            text("Loading...").into()
        } else {
            row![
                button("Back").on_press(Message::ToPreviousStep),
                button("Start").on_press(Message::ToApp)
            ]
            .spacing(10)
            .into()
        };

        let view: Element<_> = match self.step {
            0 => {
                let content = column![
                    container(title(title_1))
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(
                        row![
                            container(welcome_image)
                                .max_width(WELCOME_IMAGE_MAX_WIDTH)
                                .height(Length::Fill),
                            container(text(text_1a).size(WELCOME_TEXT_SIZE))
                                .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                        ]
                        .spacing(20)
                    )
                    .height(Length::FillPortion(4))
                    .width(Length::Fill)
                    .center_y()
                    .center_x(),
                    container(
                        column![
                            container(make_dots(self.step))
                                .center_x()
                                .width(Length::Fill),
                            container(button("Next").on_press(Message::ToNextStep))
                                .center_x()
                                .width(Length::Fill)
                        ]
                        .spacing(5)
                    )
                    .height(Length::FillPortion(1))
                ]
                .spacing(30);
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg1)
                    .into()
            }
            1 => {
                let content = column![
                    container(title(title_2))
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(
                        row![
                            container(relays_image)
                                .max_width(WELCOME_IMAGE_MAX_WIDTH)
                                .height(Length::Fill),
                            container(
                                column![
                                    container(text(text_2).size(WELCOME_TEXT_SIZE))
                                        .width(Length::Fill)
                                        .center_x(),
                                    relays_sugestions
                                ]
                                .spacing(10)
                            )
                            .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                            .height(Length::Fill),
                        ]
                        .spacing(20)
                    )
                    .height(Length::FillPortion(4))
                    .width(Length::Fill)
                    .center_y()
                    .center_x(),
                    container(
                        column![
                            container(make_dots(self.step))
                                .center_x()
                                .width(Length::Fill),
                            container(
                                row![
                                    button("Back").on_press(Message::ToPreviousStep),
                                    button("Next").on_press(Message::ToNextStep)
                                ]
                                .spacing(10)
                            )
                            .center_x()
                            .width(Length::Fill)
                        ]
                        .spacing(5)
                    )
                    .height(Length::FillPortion(1))
                ]
                .spacing(10);
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg2)
                    .into()
            }
            _ => {
                let content = column![
                    container(title(title_3))
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(
                        row![
                            container(contacts_image)
                                .max_width(WELCOME_IMAGE_MAX_WIDTH)
                                .height(Length::Fill),
                            container(
                                column![
                                    button(
                                        container(text(text_3a).size(WELCOME_TEXT_SIZE))
                                            .padding(30)
                                    )
                                    .style(style::Button::Bordered)
                                    .on_press(Message::ImportContactsRelays),
                                    button(
                                        container(text(text_3b).size(WELCOME_TEXT_SIZE))
                                            .padding(30)
                                    )
                                    .style(style::Button::Bordered)
                                    .on_press(Message::ImportContactsFile),
                                ]
                                .spacing(10)
                            )
                            .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                            .height(Length::Fill)
                            .center_x()
                            .center_y(),
                        ]
                        .spacing(20)
                    )
                    .height(Length::FillPortion(4))
                    .width(Length::Fill)
                    .center_y()
                    .center_x(),
                    container(
                        column![
                            container(make_dots(self.step))
                                .center_x()
                                .width(Length::Fill),
                            container(last_step_buttons).center_x().width(Length::Fill)
                        ]
                        .spacing(5)
                    )
                    .height(Length::FillPortion(1))
                ]
                .spacing(10);

                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg3)
                    .into()
            }
        };

        view.into()
    }
}

fn make_dots(step: usize) -> Element<'static, Message> {
    let mut dot_row = row![].spacing(5);

    for i in 0..3 {
        if i < step + 1 {
            dot_row = dot_row.push(solid_circle_icon().size(10));
        } else {
            dot_row = dot_row.push(regular_circle_icon().size(10));
        }
    }

    dot_row.width(Length::Shrink).into()
}

const WELCOME_IMAGE_MAX_WIDTH: f32 = 300.0;
const WELCOME_TEXT_SIZE: u16 = 24;
const WELCOME_TEXT_WIDTH: f32 = 400.0;
