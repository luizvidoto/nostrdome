use iced::widget::{button, column, container, row, text, text_input, tooltip, Space};
use iced::{Alignment, Length};

use crate::components::{common_scrollable, contact_row, ContactRow};
use crate::db::{DbRelay, DbRelayResponse};
use crate::icon::{import_icon, plus_icon, refresh_icon, satellite_icon, to_cloud_icon};
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::utils::contact_matches_search_full;
use crate::views::RouterMessage;
use crate::widget::Element;
use crate::{components::text::title, db::DbContact};

use super::SettingsRouterMessage;

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(BackendEvent),
    DeleteContact(DbContact),
    OpenProfileModal(DbContact),
    ContactRowMessage(contact_row::Message),
    OpenAddContactModal,
    OpenImportContactModal,
    SearchContactInputChange(String),
    RefreshContacts,
    SendContactList,
    RelaysConfirmationPress(Option<ContactsRelaysResponse>),
    SendMessageTo(DbContact),
}

#[derive(Debug, Clone)]
pub struct ContactsRelaysResponse {
    pub confirmed_relays: Vec<DbRelayResponse>,
    pub all_relays: Vec<DbRelay>,
}
impl ContactsRelaysResponse {
    fn new(
        confirmed_relays: Vec<DbRelayResponse>,
        all_relays: Vec<DbRelay>,
    ) -> ContactsRelaysResponse {
        Self {
            confirmed_relays,
            all_relays,
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    search_contact_input: String,
    relays_response: Option<ContactsRelaysResponse>,
    contact_list_changed: bool,
}
impl State {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::ToBackend::FetchContacts);
        conn.send(net::ToBackend::FetchRelayResponsesContactList);
        Self {
            contacts: vec![],
            search_contact_input: "".into(),
            relays_response: None,
            contact_list_changed: false,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Option<SettingsRouterMessage> {
        match message {
            Message::SendMessageTo(_) => (),
            Message::RelaysConfirmationPress(_) => (),
            Message::SendContactList => {
                conn.send(net::ToBackend::SendContactListToRelays);
            }
            Message::RefreshContacts => {
                conn.send(net::ToBackend::RefreshContactsProfile);
            }
            Message::OpenProfileModal(db_contact) => {
                return Some(SettingsRouterMessage::OpenProfileModal(db_contact))
            }
            Message::OpenAddContactModal => {
                return Some(SettingsRouterMessage::OpenAddContactModal)
            }
            Message::OpenImportContactModal => {
                return Some(SettingsRouterMessage::OpenImportContactModal)
            }
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ContactRowMessage(ct_msg) => match ct_msg {
                // TODO: dont return a message, find a better way
                contact_row::Message::SendMessageTo(contact) => {
                    return Some(SettingsRouterMessage::RouterMessage(
                        RouterMessage::GoToChatTo(contact),
                    ));
                }
                contact_row::Message::DeleteContact(contact) => {
                    conn.send(net::ToBackend::DeleteContact(contact))
                }
                contact_row::Message::EditContact(contact) => {
                    return Some(SettingsRouterMessage::OpenEditContactModal(contact));
                }
            },
            Message::DeleteContact(contact) => {
                conn.send(net::ToBackend::DeleteContact(contact));
            }
            Message::BackEndEvent(event) => match event {
                BackendEvent::ConfirmedContactList(_) => {
                    conn.send(net::ToBackend::FetchRelayResponsesContactList);
                    self.contact_list_changed = false;
                }
                BackendEvent::GotRelayResponsesContactList {
                    responses,
                    all_relays,
                } => {
                    self.relays_response = Some(ContactsRelaysResponse::new(responses, all_relays));
                }
                BackendEvent::GotContacts(db_contacts) => {
                    self.contacts = db_contacts;
                }
                BackendEvent::UpdatedContactMetadata { db_contact, .. } => {
                    if let Some(contact) = self
                        .contacts
                        .iter_mut()
                        .find(|c| c.pubkey() == db_contact.pubkey())
                    {
                        *contact = db_contact;
                    }
                }
                BackendEvent::ReceivedContactList { .. } => {
                    conn.send(net::ToBackend::FetchContacts);
                }
                BackendEvent::FileContactsImported(_)
                | BackendEvent::ContactCreated(_)
                | BackendEvent::ContactUpdated(_)
                | BackendEvent::ContactDeleted(_) => {
                    conn.send(net::ToBackend::FetchContacts);
                    self.contact_list_changed = true;
                }
                _ => (),
            },
        }

        None
    }

    fn make_relays_response<'a>(&self) -> Element<'a, Message> {
        if let Some(response) = &self.relays_response {
            let resp_txt = format!(
                "{}/{}",
                &response.confirmed_relays.len(),
                &response.all_relays.len()
            );
            tooltip(
                button(
                    row![
                        text(&resp_txt).size(18).style(style::Text::Placeholder),
                        satellite_icon().size(16).style(style::Text::Placeholder)
                    ]
                    .spacing(5),
                )
                .on_press(Message::RelaysConfirmationPress(
                    self.relays_response.to_owned(),
                ))
                .style(style::Button::MenuBtn)
                .padding(5),
                "Relays Confirmation",
                tooltip::Position::Left,
            )
            .style(style::Container::TooltipBg)
            .into()
        } else {
            text("Contact list not confirmed on any relays")
                .size(18)
                .style(style::Text::Placeholder)
                .into()
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Contacts");
        let title_group = row![
            title,
            Space::with_width(Length::Fill),
            self.make_relays_response()
        ]
        .align_items(Alignment::Center)
        .spacing(10);

        let search_contact = text_input("Search", &self.search_contact_input)
            .on_input(Message::SearchContactInputChange)
            .style(style::TextInput::ChatSearch)
            .width(SEARCH_CONTACT_WIDTH);
        let add_contact_btn = tooltip(
            button(
                row![text("Add").size(18), plus_icon().size(14)]
                    .align_items(Alignment::Center)
                    .spacing(2),
            )
            .padding(5)
            .on_press(Message::OpenAddContactModal),
            "Add Contact",
            tooltip::Position::Top,
        )
        .style(style::Container::TooltipBg);
        let refresh_btn = tooltip(
            button(refresh_icon().size(18)).on_press(Message::RefreshContacts),
            "Refresh Contacts",
            tooltip::Position::Top,
        )
        .style(style::Container::TooltipBg);
        let import_btn = tooltip(
            button(import_icon().size(18))
                .padding(5)
                .on_press(Message::OpenImportContactModal),
            "Import from file",
            tooltip::Position::Top,
        )
        .style(style::Container::TooltipBg);
        let send_btn = tooltip(
            button(to_cloud_icon().size(18))
                .padding(5)
                .on_press(Message::SendContactList),
            "Send to relays",
            tooltip::Position::Top,
        )
        .style(style::Container::TooltipBg);

        let utils_row = row![
            search_contact,
            Space::with_width(Length::Fill),
            add_contact_btn,
            refresh_btn,
            import_btn,
            send_btn
        ]
        .spacing(5)
        .width(Length::Fill);

        let contact_list: Element<_> = self
            .contacts
            .iter()
            .filter(|c| contact_matches_search_full(c, &self.search_contact_input))
            .map(|c| ContactRow::from_db_contact(c))
            .fold(column![].spacing(5), |col, contact| {
                col.push(contact.view().map(Message::ContactRowMessage))
            })
            .into();
        let contact_list_scroller = column![ContactRow::header(), common_scrollable(contact_list)];
        let content: Element<_> = column![title_group, utils_row, contact_list_scroller]
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        container(content).center_x().center_y().into()
    }
}

const SEARCH_CONTACT_WIDTH: f32 = 200.0;
