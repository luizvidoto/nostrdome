use std::collections::HashMap;
use std::fmt::Debug;

use crate::components::text::title;
use crate::components::{common_scrollable, copy_btn};
use crate::db::DbRelay;
use crate::net::{BackEndConnection, BackendEvent, ToBackend};
use crate::style;
use crate::utils::NipData;
use crate::widget::Element;
use iced::widget::{button, column, container, row, text, Rule};
use iced::{alignment, clipboard};
use iced::{Alignment, Command, Length};
use iced_aw::{Card, Modal};
use url::Url;

pub struct RelayDocState {
    db_relay: DbRelay,
    nips_data: HashMap<u16, NipData>,
}

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    UnderlayMessage(M),
    CloseModal,
    Copy(String),
    OpenLink(String),
}
impl RelayDocState {
    pub fn new(db_relay: DbRelay, conn: &mut BackEndConnection) -> Self {
        conn.send(ToBackend::FetchNipsData);
        Self {
            db_relay,
            nips_data: HashMap::new(),
        }
    }

    pub(crate) fn backend_event(&mut self, event: BackendEvent, _conn: &mut BackEndConnection) {
        match event {
            BackendEvent::GotNipsData(nips) => {
                let mut nips_data = HashMap::new();
                for data in nips {
                    nips_data.insert(data.number, data);
                }
                self.nips_data = nips_data;
            }
            _ => (),
        }
    }
    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, move || {
            let page_title = title("Relay Document");
            let page_subtitle = text(&self.db_relay.url.to_string()).size(24);

            let modal_body: Element<_> = if let Some(information) = &self.db_relay.information {
                if let Some(document) = &information.document {
                    let name_gp = column![
                        text("Name").size(24),
                        text(&document.name.as_ref().unwrap_or(&"".into())),
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    let description_gp = column![
                        text("Description").size(24),
                        text(&document.description.as_ref().unwrap_or(&"".into())),
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    let pubkey_text: Element<_> = if let Some(pubkey) = &document.pubkey {
                        text_and_copy_btn(&pubkey, CMessage::Copy(pubkey.to_string()))
                    } else {
                        text("").into()
                    };
                    let pubkey_gp = column![
                        text("Public Key").size(24),
                        pubkey_text,
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    let contact_text = if let Some(contact) = &document.contact {
                        text_and_copy_btn(&contact, CMessage::Copy(contact.to_string()))
                    } else {
                        text("").into()
                    };
                    let contact_gp =
                        column![text("Contact").size(24), contact_text, Rule::horizontal(5),]
                            .spacing(5);

                    let supported_nips_col: Element<_> =
                        if let Some(nips) = document.supported_nips.as_ref() {
                            nips.iter()
                                .filter_map(|nip| self.nips_data.get(nip))
                                .fold(column![].spacing(5), |col, nip| col.push(nip_row(nip)))
                                .into()
                        } else {
                            text("Not informed").into()
                        };
                    let nips_gp = column![
                        text("Supported Nips").size(24),
                        supported_nips_col,
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    let software_text = if let Some(software) = &document.software {
                        text_and_copy_btn(&software, CMessage::Copy(software.to_string()))
                    } else {
                        text("").into()
                    };
                    let software_gp = column![
                        text("Software").size(24),
                        software_text,
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    let version_gp = column![
                        text("Version").size(24),
                        text(&document.version.as_ref().unwrap_or(&"".into())),
                        Rule::horizontal(5),
                    ]
                    .spacing(5);

                    column![
                        name_gp,
                        description_gp,
                        pubkey_gp,
                        contact_gp,
                        nips_gp,
                        software_gp,
                        version_gp
                    ]
                    .spacing(10)
                    .into()
                } else {
                    text("Relay has no document").into()
                }
            } else {
                text("Relay has no information").into()
            };

            let modal_foot =
                row![
                    button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(CMessage::CloseModal)
                ]
                .spacing(10)
                .height(50)
                .width(Length::Fill);
            let modal_body = container(common_scrollable(modal_body));
            let modal_head = column![page_title, page_subtitle].spacing(5).height(100);

            Card::new(modal_head, modal_body)
                .foot(modal_foot)
                .max_height(MODAL_MAX_HEIGHT)
                .max_width(MODAL_WIDTH)
                .on_close(CMessage::CloseModal)
                .into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }

    /// Returns true if modal is to be closed
    pub(crate) fn update<'a, M: 'a + Clone + Debug>(
        &mut self,
        message: CMessage<M>,
        _conn: &mut BackEndConnection,
    ) -> (Command<CMessage<M>>, bool) {
        let mut command = Command::none();
        match message {
            CMessage::UnderlayMessage(_) => (),
            CMessage::Copy(text) => {
                command = clipboard::write(text);
            }
            CMessage::CloseModal => {
                return (command, true);
            }
            CMessage::OpenLink(link) => match Url::parse(&link) {
                Ok(parsed_link) => {
                    if let Err(e) = webbrowser::open(parsed_link.as_str()) {
                        tracing::error!("Failed to open browser: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to open link: {}", e);
                }
            },
        }
        (command, false)
    }
}

fn text_and_copy_btn<'a, M: 'a + Clone + Debug>(
    display_text: &'a str,
    message: CMessage<M>,
) -> Element<'a, CMessage<M>> {
    container(
        row![
            container(text(display_text)).width(Length::Fill),
            copy_btn("Copy", message)
        ]
        .align_items(Alignment::Center)
        .spacing(5),
    )
    .padding([2, 4, 2, 0])
    .width(Length::Fill)
    .into()
}

fn nip_row<'a, M: 'a + Clone + Debug>(nip: &'a NipData) -> Element<CMessage<M>> {
    row![
        text(&format!("NIP-{:02}", &nip.number)),
        button(text(&nip.description))
            .padding(0)
            .style(style::Button::Link)
            .on_press(CMessage::OpenLink(nip.repo_link.to_owned()))
    ]
    .spacing(5)
    .into()
}

const MODAL_WIDTH: f32 = 500.0;
const MODAL_MAX_HEIGHT: f32 = 300.0;
