use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use iced_aw::{Card, Modal};

use crate::components::text_input_group::text_input_group;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    CloseAddContactModal,
    SubmitNewContact,
    OpenAddContactModal,
    DbEvent(net::database::Event),
    ModalPetNameInputChange(String),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Loaded {
        contacts: Vec<DbContact>,
        show_modal: bool,
        modal_petname_input: String,
    },
}
impl State {
    pub fn default() -> Self {
        Self::Loading
    }
    pub fn loaded(contacts: &[DbContact]) -> Self {
        Self::Loaded {
            contacts: contacts.to_vec(),
            show_modal: false,
            modal_petname_input: "".into(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::OpenAddContactModal => {
                if let Self::Loaded { mut show_modal, .. } = self {
                    show_modal = true;
                }
            }
            Message::CloseAddContactModal => {
                if let Self::Loaded { mut show_modal, .. } = self {
                    show_modal = false;
                }
            }
            Message::DbEvent(_ev) => (),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Contacts");
        let table_header = row![
            text("petname").width(Length::Fill),
            text("pubkey").width(Length::Fill),
            text("recommended_relay").width(Length::Fill),
            text("profile_image").width(Length::Fill),
        ];
        let add_contact_btn = button("Add").on_press(Message::OpenAddContactModal);
        let content: Element<_> = match self {
            State::Loading => title.into(),
            State::Loaded { contacts, .. } => column![title, table_header, add_contact_btn]
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        };
        match self {
            State::Loading => content,
            State::Loaded {
                contacts,
                show_modal,
                modal_petname_input,
            } => {
                Modal::new(*show_modal, content, || {
                    let add_relay_input = text_input_group(
                        "Contact Name",
                        "",
                        &modal_petname_input,
                        None,
                        Message::ModalPetNameInputChange,
                    );
                    let modal_body: Element<_> = container(add_relay_input).into();
                    Card::new(text("Add Relay"), modal_body)
                        .foot(
                            row![
                                button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::CloseAddContactModal),
                                button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                    .width(Length::Fill)
                                    .on_press(Message::SubmitNewContact)
                            ]
                            .spacing(10)
                            .padding(5)
                            .width(Length::Fill),
                        )
                        .max_width(300.0)
                        //.width(Length::Shrink)
                        .on_close(Message::CloseAddContactModal)
                        .into()
                })
                .backdrop(Message::CloseAddContactModal)
                .on_esc(Message::CloseAddContactModal)
                .into()
            }
        }
    }
}
