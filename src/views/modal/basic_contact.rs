use std::fmt::Debug;

use crate::components::text_input_group::TextInputGroup;
use crate::components::{card, common_scrollable};
use crate::consts::{MEDIUM_PROFILE_IMG_HEIGHT, MEDIUM_PROFILE_IMG_WIDTH, YMD_FORMAT};
use crate::db::DbContact;
use crate::error::BackendClosed;
use crate::icon::{copy_icon, edit_icon};
use crate::net::{self, BackEndConnection, BackendEvent, ImageSize};
use crate::utils::{from_naive_utc_to_local, hide_string};
use iced::widget::{button, column, container, image, row, text, tooltip, Space};
use iced::{alignment, clipboard};
use iced::{Alignment, Command, Length};
use iced_aw::Modal;
use nostr::prelude::ToBech32;

use crate::style;
use crate::widget::{Element, Rule};

use super::ModalView;

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
pub struct ContactDetails<M: Clone + Debug> {
    db_contact: Option<DbContact>,
    petname_input: String,
    pubkey_input: String,
    rec_relay_input: String,
    mode: Mode,
    is_pub_invalid: bool,
    is_relay_invalid: bool,
    profile_img_handle: Option<image::Handle>,
    pubkey_hidden: String,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Clone + Debug> ContactDetails<M> {
    pub(crate) fn new() -> Self {
        Self {
            db_contact: None,
            petname_input: "".into(),
            pubkey_input: "".into(),
            rec_relay_input: "".into(),
            mode: Mode::Add,
            is_pub_invalid: false,
            is_relay_invalid: false,
            profile_img_handle: None,
            pubkey_hidden: "".into(),
            phantom: std::marker::PhantomData,
        }
    }
    pub(crate) fn edit(
        db_contact: &DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        let pubkey_input = db_contact
            .pubkey()
            .to_bech32()
            .unwrap_or(db_contact.pubkey().to_string());
        Ok(Self {
            pubkey_hidden: hide_string(&pubkey_input, 16),
            db_contact: Some(db_contact.to_owned()),
            petname_input: db_contact.get_petname().unwrap_or_else(|| "".into()),
            pubkey_input,
            rec_relay_input: db_contact
                .get_relay_url()
                .map(|url| url.to_string())
                .unwrap_or("".into()),
            mode: Mode::Edit,
            is_pub_invalid: false,
            is_relay_invalid: false,
            profile_img_handle: Some(db_contact.profile_image(ImageSize::Medium, conn)?),
            phantom: std::marker::PhantomData,
        })
    }
    pub(crate) fn viewer(
        db_contact: &DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        let mut details = Self::edit(db_contact, conn)?;
        details.mode = Mode::View;
        Ok(details)
    }

    pub(crate) fn handle_submit_contact(
        &mut self,
        conn: &mut BackEndConnection,
    ) -> Result<bool, BackendClosed> {
        let submit_result = match &self.db_contact {
            Some(db_contact) => DbContact::edit_contact(
                db_contact.to_owned(),
                &self.petname_input,
                &self.rec_relay_input,
            ),
            None => DbContact::new_from_submit(
                &self.pubkey_input,
                &self.petname_input,
                &self.rec_relay_input,
            ),
        };

        match submit_result {
            Ok(db_contact) => {
                match self.mode {
                    Mode::Edit => conn.send(net::ToBackend::UpdateContact(db_contact))?,
                    Mode::Add => conn.send(net::ToBackend::AddContact(db_contact))?,
                    Mode::View => Result::Ok(())?,
                }

                // *self = Self::Off;
                return Ok(true);
            }
            Err(e) => {
                tracing::error!("Error: {:?}", e);
                match e {
                    crate::db::contact::Error::InvalidPublicKey => {
                        self.is_pub_invalid = true;
                    }
                    crate::db::contact::Error::InvalidRelayUrl(_) => {
                        self.is_relay_invalid = true;
                    }
                    _ => (),
                }
            }
        }

        Ok(false)
    }
}

impl<M: Clone + Debug + 'static + Send> ModalView for ContactDetails<M> {
    type UnderlayMessage = M;
    type Message = CMessage<M>;

    fn view<'a>(
        &'a self,
        underlay: impl Into<Element<'a, Self::UnderlayMessage>>,
    ) -> Element<'a, Self::Message> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, move || {
            let modal_body: Element<_> = match self.mode {
                Mode::Add | Mode::Edit => {
                    let petname_input = TextInputGroup::new(
                        "Contact Name",
                        &self.petname_input,
                        CMessage::PetNameInputChange,
                    )
                    .placeholder("");

                    let mut pubkey_input = TextInputGroup::new(
                        "Contact PubKey",
                        &self.pubkey_input,
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
                        &self.rec_relay_input,
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
                    let petname_text: &str = if self.petname_input.is_empty() {
                        "No name"
                    } else {
                        &self.petname_input
                    };
                    let rec_relay_text: &str = if self.rec_relay_input.is_empty() {
                        "No relay set"
                    } else {
                        &self.rec_relay_input
                    };
                    let petname_group = column![
                        text("Contact Name"),
                        container(text(petname_text))
                            .padding([2, 8])
                            .style(style::Container::Frame),
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
                                container(text(&self.pubkey_hidden)).width(Length::Fill),
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
                            .style(style::Container::Frame),
                    ]
                    .spacing(2);
                    let middle = column![pubkey_group, petname_group, relay_group].spacing(4);
                    let profile_top = make_profile_top_row(
                        self.db_contact.as_ref(),
                        self.profile_img_handle.as_ref(),
                        CMessage::EditMode,
                    );
                    column![profile_top, middle,].spacing(4).into()
                }
            };
            let modal_body = common_scrollable(container(modal_body).padding(20));

            let delete_btn: Element<_> = match self.mode {
                Mode::Add => Space::with_width(Length::Fill).into(),
                _ => button(text("Delete").horizontal_alignment(alignment::Horizontal::Center))
                    .width(Length::Fill)
                    .on_press(CMessage::DeleteContact)
                    .style(style::Button::Danger)
                    .into(),
            };

            let cancel_btn: Element<_> = match self.mode {
                Mode::View => Space::with_width(Length::Fill).into(),
                _ => button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center))
                    .style(style::Button::Bordered)
                    .width(Length::Fill)
                    .on_press(CMessage::CloseModal)
                    .into(),
            };

            let buttons_row = row![
                delete_btn,
                cancel_btn,
                button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                    .style(style::Button::Primary)
                    .width(Length::Fill)
                    .on_press(CMessage::SubmitContact)
            ]
            .spacing(10)
            .width(Length::Fill);

            card(modal_body, buttons_row).max_width(MODAL_WIDTH).into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        if let BackendEvent::ImageDownloaded(image) = event {
            if let Some(db_contact) = &self.db_contact {
                if db_contact.get_profile_event_hash() == Some(image.event_hash) {
                    self.profile_img_handle =
                        Some(db_contact.profile_image(ImageSize::Medium, conn)?)
                }
            }
        }
        Ok(())
    }

    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> Result<(Command<Self::Message>, bool), BackendClosed> {
        let mut command = Command::none();
        match message {
            CMessage::DeleteContact => {
                if let Some(contact) = &self.db_contact {
                    conn.send(net::ToBackend::DeleteContact(contact.to_owned()))?;
                }
                return Ok((command, true));
            }
            CMessage::CopyPubkey => {
                command = clipboard::write(self.pubkey_input.to_owned());
            }
            CMessage::EditMode => {
                if let Mode::View = self.mode {
                    self.mode = Mode::Edit;
                }
            }
            CMessage::PetNameInputChange(text) => {
                self.petname_input = text;
            }
            CMessage::PubKeyInputChange(text) => {
                self.pubkey_input = text;
                self.is_pub_invalid = false;
            }
            CMessage::RecRelayInputChange(text) => {
                self.rec_relay_input = text;
                self.is_relay_invalid = false;
            }
            CMessage::SubmitContact => {
                let is_close = self.handle_submit_contact(conn)?;
                return Ok((command, is_close));
            }
            CMessage::CloseModal => {
                return Ok((command, true));
            }
            CMessage::UnderlayMessage(_) => (),
        }

        Ok((command, false))
    }
}

fn make_profile_top_row<'a, M: 'a + Clone>(
    db_contact: Option<&'a DbContact>,
    img_handle: Option<&image::Handle>,
    edit_press: M,
) -> Element<'a, M> {
    if let Some(contact) = db_contact {
        if let Some(profile) = contact.get_profile_cache() {
            let image_container: Element<_> = if let Some(handle) = img_handle {
                image(handle.to_owned()).into()
            } else {
                text("No image").into()
            };

            let image_container = container(image_container)
                .height(MEDIUM_PROFILE_IMG_HEIGHT as f32)
                .width(MEDIUM_PROFILE_IMG_WIDTH as f32);

            let profile_name_group = column![
                text("Profile Name"),
                container(text(profile.metadata.name.unwrap_or("".into())))
                    .padding([2, 8])
                    .style(style::Container::Frame),
            ]
            .spacing(2);
            let profile_username_group = column![
                text("Profile Username"),
                container(text(profile.metadata.display_name.unwrap_or("".into())))
                    .padding([2, 8])
                    .style(style::Container::Frame),
            ]
            .spacing(2);

            let last_update_group = column![
                text("Last Update"),
                container(text(
                    &from_naive_utc_to_local(profile.updated_at).format(YMD_FORMAT)
                ))
                .padding([2, 8])
                .style(style::Container::Frame),
            ]
            .spacing(2);

            let recv_from_relay_group = column![
                text("Received From"),
                container(text(profile.from_relay))
                    .padding([2, 8])
                    .style(style::Container::Frame),
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
                .spacing(10),
                button(edit_icon().size(14)).on_press(edit_press)
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
    }
}

const MODAL_WIDTH: f32 = 500.0;
const COPY_BTN_WIDTH: f32 = 30.0;
