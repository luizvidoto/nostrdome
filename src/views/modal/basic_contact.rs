use std::fmt::Debug;

use crate::components::common_scrollable;
use crate::components::text_input_group::TextInputGroup;
use crate::consts::{MEDIUM_PROFILE_PIC_HEIGHT, MEDIUM_PROFILE_PIC_WIDTH, YMD_FORMAT};
use crate::db::{DbContact, DbContactError};
use crate::icon::{copy_icon, edit_icon};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection, ImageSize};
use crate::utils::from_naive_utc_to_local;
use iced::widget::{button, column, container, image, row, text, tooltip, Space};
use iced::{alignment, clipboard};
use iced::{Alignment, Command, Length};
use iced_aw::{Card, Modal};

use crate::style;
use crate::widget::{Element, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    View,
    Add,
}

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    PetNameInputChange(String),
    PubKeyInputChange(String),
    RecRelayInputChange(String),
    SubmitContact,
    CloseModal,
    EditMode,
    UnderlayMessage(M),
    CopyPubkey,
    DeleteContact,
}
#[derive(Debug, Clone)]
pub struct ContactDetails {
    db_contact: Option<DbContact>,
    modal_petname_input: String,
    modal_pubkey_input: String,
    modal_rec_relay_input: String,
    mode: Mode,
    is_pub_invalid: bool,
    is_relay_invalid: bool,
    profile_img_handle: Option<image::Handle>,
}
impl ContactDetails {
    pub(crate) fn new() -> Self {
        Self {
            db_contact: None,
            modal_petname_input: "".into(),
            modal_pubkey_input: "".into(),
            modal_rec_relay_input: "".into(),
            mode: Mode::Add,
            is_pub_invalid: false,
            is_relay_invalid: false,
            profile_img_handle: None,
        }
    }
    pub(crate) fn edit(db_contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        Self {
            db_contact: Some(db_contact.to_owned()),
            modal_petname_input: db_contact.get_petname().unwrap_or_else(|| "".into()),
            modal_pubkey_input: db_contact.pubkey().to_string(),
            modal_rec_relay_input: db_contact
                .get_relay_url()
                .map(|url| url.to_string())
                .unwrap_or("".into()),
            mode: Mode::Edit,
            is_pub_invalid: false,
            is_relay_invalid: false,
            profile_img_handle: Some(db_contact.profile_image(ImageSize::Medium, conn)),
        }
    }
    pub(crate) fn viewer(db_contact: &DbContact, conn: &mut BackEndConnection) -> Self {
        if let Some(cache) = db_contact.get_profile_cache() {
            conn.send(net::ToBackend::GetDbEventWithHash(cache.event_hash));
        }
        let mut details = Self::edit(db_contact, conn);
        details.mode = Mode::View;
        details
    }
    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, move || {
            let modal_body: Element<_> = match self.mode {
                Mode::Add | Mode::Edit => {
                    let petname_input = TextInputGroup::new(
                        "Contact Name",
                        &self.modal_petname_input,
                        CMessage::PetNameInputChange,
                    )
                    .placeholder("");

                    let mut pubkey_input = TextInputGroup::new(
                        "Contact PubKey",
                        &self.modal_pubkey_input,
                        CMessage::PubKeyInputChange,
                    )
                    .placeholder("");

                    if self.is_pub_invalid {
                        pubkey_input = pubkey_input.invalid("Invalid Public Key");
                    }

                    if let Mode::Edit = self.mode {
                        pubkey_input = pubkey_input.disabled();
                    }

                    let mut rec_relay_input = TextInputGroup::new(
                        "Recommended Relay",
                        &self.modal_rec_relay_input,
                        CMessage::RecRelayInputChange,
                    )
                    .placeholder("ws://my-relay.com");

                    if self.is_relay_invalid {
                        rec_relay_input = rec_relay_input.invalid("Invalid Relay URL");
                    }

                    column![
                        pubkey_input.build(),
                        petname_input.build(),
                        rec_relay_input.build()
                    ]
                    .spacing(4)
                    .into()
                }
                Mode::View => {
                    let petname_text: &str = if self.modal_petname_input.is_empty() {
                        "No name"
                    } else {
                        &self.modal_petname_input
                    };
                    let rec_relay_text: &str = if self.modal_rec_relay_input.is_empty() {
                        "No relay set"
                    } else {
                        &self.modal_rec_relay_input
                    };
                    let petname_group = column![
                        text("Contact Name"),
                        container(text(petname_text))
                            .padding([2, 8])
                            .style(style::Container::ChatSearchCopy),
                    ]
                    .spacing(2);
                    let copy_btn = tooltip(
                        button(copy_icon())
                            .on_press(CMessage::CopyPubkey)
                            .style(style::Button::MenuBtn)
                            .width(COPY_BTN_WIDTH),
                        "Copy",
                        tooltip::Position::Top,
                    )
                    .style(style::Container::TooltipBg);

                    let pubkey_group = column![
                        text("Contact PubKey"),
                        container(
                            row![
                                container(text(&self.modal_pubkey_input)).width(Length::Fill),
                                copy_btn
                            ]
                            .align_items(Alignment::Center)
                            .spacing(5)
                        )
                        .padding([2, 4, 2, 8])
                        .width(Length::Fill),
                    ]
                    .spacing(2);
                    let relay_group = column![
                        text("Recommended Relay"),
                        container(text(rec_relay_text))
                            .padding([2, 8])
                            .style(style::Container::ChatSearchCopy),
                    ]
                    .spacing(2);
                    let middle = column![pubkey_group, petname_group, relay_group].spacing(4);

                    let profile_top: Element<_> = if let Some(contact) = &self.db_contact {
                        if let Some(profile) = contact.get_profile_cache() {
                            let image_container: Element<_> =
                                if let Some(handle) = self.profile_img_handle.to_owned() {
                                    image(handle).into()
                                } else {
                                    text("No image").into()
                                };
                            let image_container = container(image_container)
                                .height(MEDIUM_PROFILE_PIC_HEIGHT as f32)
                                .width(MEDIUM_PROFILE_PIC_WIDTH as f32);

                            let profile_name_group = column![
                                text("Profile Name"),
                                container(text(profile.metadata.name.unwrap_or("".into())))
                                    .padding([2, 8])
                                    .style(style::Container::ChatSearchCopy),
                            ]
                            .spacing(2);
                            let profile_username_group = column![
                                text("Profile Username"),
                                container(text(profile.metadata.display_name.unwrap_or("".into())))
                                    .padding([2, 8])
                                    .style(style::Container::ChatSearchCopy),
                            ]
                            .spacing(2);

                            let last_update_group = column![
                                text("Last Update"),
                                container(text(
                                    &from_naive_utc_to_local(profile.updated_at).format(YMD_FORMAT)
                                ))
                                .padding([2, 8])
                                .style(style::Container::ChatSearchCopy),
                            ]
                            .spacing(2);

                            let recv_from_relay_group = column![
                                text("Received From"),
                                container(text(profile.from_relay))
                                    .padding([2, 8])
                                    .style(style::Container::ChatSearchCopy),
                            ]
                            .spacing(2);

                            let image_row = row![
                                image_container,
                                column![
                                    profile_name_group,
                                    profile_username_group,
                                    last_update_group,
                                    recv_from_relay_group
                                ]
                                .padding([0, 10])
                                .spacing(10)
                            ];
                            let divider = container(Rule::horizontal(2))
                                .padding(10)
                                .width(Length::Fill);
                            column![image_row, divider].into()
                        } else {
                            text("").into()
                        }
                    } else {
                        text("").into()
                    };

                    column![profile_top, middle,].spacing(4).into()
                }
            };
            let modal_body = common_scrollable(modal_body);

            let title: Element<_> = match self.mode {
                Mode::Edit => text("Edit Contact").into(),
                Mode::Add => text("Add Contact").into(),
                Mode::View => row![
                    text("Contact"),
                    Space::with_width(Length::Fill),
                    button(edit_icon().size(14)).on_press(CMessage::EditMode)
                ]
                .into(),
            };

            Card::new(title, modal_body)
                .foot(
                    row![
                        button(text("Delete").horizontal_alignment(alignment::Horizontal::Center),)
                            .width(Length::Fill)
                            .on_press(CMessage::DeleteContact)
                            .style(style::Button::Danger),
                        button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center),)
                            .width(Length::Fill)
                            .on_press(CMessage::CloseModal),
                        button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                            .width(Length::Fill)
                            .on_press(CMessage::SubmitContact)
                    ]
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill),
                )
                .max_width(MODAL_WIDTH)
                .on_close(CMessage::CloseModal)
                .into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }

    pub fn backend_event(&mut self, event: Event, _conn: &mut BackEndConnection) {
        match event {
            _ => (),
        }
    }

    /// Returns true if modal is to be closed
    pub(crate) fn update<'a, M: 'a + Clone + Debug>(
        &mut self,
        message: CMessage<M>,
        conn: &mut BackEndConnection,
    ) -> (Command<CMessage<M>>, bool) {
        let mut command = Command::none();
        match message {
            CMessage::DeleteContact => {
                if let Some(contact) = &self.db_contact {
                    conn.send(net::ToBackend::DeleteContact(contact.to_owned()));
                }
                return (command, true);
            }
            CMessage::CopyPubkey => {
                command = clipboard::write(self.modal_pubkey_input.to_owned());
            }
            CMessage::EditMode => {
                if let Mode::View = self.mode {
                    self.mode = Mode::Edit;
                }
            }
            CMessage::PetNameInputChange(text) => {
                self.modal_petname_input = text;
            }
            CMessage::PubKeyInputChange(text) => {
                self.modal_pubkey_input = text;
                self.is_pub_invalid = false;
            }
            CMessage::RecRelayInputChange(text) => {
                self.modal_rec_relay_input = text;
                self.is_relay_invalid = false;
            }
            CMessage::SubmitContact => return (command, self.handle_submit_contact(conn)),
            CMessage::CloseModal => {
                return (command, true);
            }
            CMessage::UnderlayMessage(_) => (),
        }
        (command, false)
    }

    pub(crate) fn handle_submit_contact(&mut self, conn: &mut BackEndConnection) -> bool {
        let submit_result = match &self.db_contact {
            Some(db_contact) => DbContact::edit_contact(
                db_contact.to_owned(),
                &self.modal_petname_input,
                &self.modal_rec_relay_input,
            ),
            None => DbContact::new_from_submit(
                &self.modal_pubkey_input,
                &self.modal_petname_input,
                &self.modal_rec_relay_input,
            ),
        };

        match submit_result {
            Ok(db_contact) => {
                match self.mode {
                    Mode::Edit => conn.send(net::ToBackend::UpdateContact(db_contact)),
                    Mode::Add => conn.send(net::ToBackend::AddContact(db_contact)),
                    Mode::View => (),
                }

                // *self = Self::Off;
                return true;
            }
            Err(e) => {
                tracing::error!("Error: {:?}", e);
                match e {
                    DbContactError::InvalidPublicKey => {
                        self.is_pub_invalid = true;
                    }
                    DbContactError::InvalidRelayUrl(_) => {
                        self.is_relay_invalid = true;
                    }
                    _ => (),
                }
            }
        }
        false
    }
}

const MODAL_WIDTH: f32 = 400.0;
const COPY_BTN_WIDTH: f32 = 30.0;
