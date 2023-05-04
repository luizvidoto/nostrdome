use iced::widget::{button, column, container, image, row, text};
use iced::Length;

use crate::consts::{CONTACTS_IMAGE, RELAYS_IMAGE, WELCOME_IMAGE};
use crate::icon::{regular_circle_icon, solid_circle_icon};
use crate::style;
use crate::{components::text::title, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    ToNextStep,
    ToPreviousStep,
    ToApp,
}
pub struct State {
    pub step: usize,
}
impl State {
    pub fn new() -> Self {
        Self { step: 0 }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToNextStep => self.step += 1,
            Message::ToPreviousStep => self.step -= 1,
            Message::ToApp => (),
        }
    }
    pub fn view(&self) -> Element<Message> {
        let title_1 = "Welcome to NostrDome";
        let text_1a = "Secure, encrypted chats on the NOSTR network.";
        let text_1b = "Protect your privacy effortlessly.";
        let title_2 = "Manage Your Relays";
        let text_2a = "NostrDome connects to multiple servers known as Relays, giving you the power to choose which servers you want to connect to.";
        let text_2b = "You can easily manage your Relays and customize your experience to suit your privacy needs.";
        let title_3 = "Handle Your Contacts";
        let text_3a = "Effortlessly manage your contacts and share them with connected Relays.";
        let text_3b = "Create backups of your sent messages, ensuring that your valuable conversations are always preserved.";

        let welcome_image = image::Image::new(image::Handle::from_memory(WELCOME_IMAGE));
        let relays_image = image::Image::new(image::Handle::from_memory(RELAYS_IMAGE));
        let contacts_image = image::Image::new(image::Handle::from_memory(CONTACTS_IMAGE));

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
                                    text(text_2a).size(WELCOME_TEXT_SIZE),
                                    text(text_2b).size(WELCOME_TEXT_SIZE)
                                ]
                                .spacing(10)
                            )
                            .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                            .height(Length::Fill)
                            .center_x()
                            .center_y(),
                            // sugestões de relays para adicionar?
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
                                    text(text_3a).size(WELCOME_TEXT_SIZE),
                                    text(text_3b).size(WELCOME_TEXT_SIZE)
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
                            container(
                                row![
                                    button("Back").on_press(Message::ToPreviousStep),
                                    button("Start").on_press(Message::ToApp)
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
