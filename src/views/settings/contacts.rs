use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{alignment, Length};

use crate::components::{common_scrollable, contact_row, ContactRow};
use crate::icon::{import_icon, plus_icon, to_cloud_icon};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::utils::contact_matches_search;
use crate::widget::Element;
use crate::{components::text::title, db::DbContact};

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(Event),
    DeleteContact(DbContact),
    ContactRowMessage(contact_row::Message),
    OpenAddContactModal,
    OpenEditContactModal(DbContact),
    OpenImportContactModal,
    SearchContactInputChange(String),
    OpenSendContactModal,
}

#[derive(Debug, Clone)]
pub struct State {
    contacts: Vec<DbContact>,
    search_contact_input: String,
}
impl State {
    pub fn new(db_conn: &mut BackEndConnection) -> Self {
        db_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            search_contact_input: "".into(),
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) -> Option<Message> {
        match message {
            Message::OpenSendContactModal => (),
            Message::OpenEditContactModal(_) => (),
            Message::OpenAddContactModal => (),
            Message::OpenImportContactModal => (),
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ContactRowMessage(ct_msg) => match ct_msg {
                contact_row::Message::DeleteContact(contact) => {
                    conn.send(net::Message::DeleteContact(contact))
                }
                contact_row::Message::EditContact(contact) => {
                    // self.modal_state = ModalState::add_contact(Some(contact));
                    return Some(Message::OpenEditContactModal(contact));
                }
                contact_row::Message::GetProfile(db_contact) => {
                    conn.send(net::Message::GetContactProfile(db_contact));
                }
            },
            Message::DeleteContact(contact) => {
                conn.send(net::Message::DeleteContact(contact));
            }
            Message::BackEndEvent(event) => match event {
                Event::GotContacts(db_contacts) => {
                    self.contacts = db_contacts;
                }
                Event::ContactsImported(_)
                | Event::ContactCreated(_)
                | Event::ContactUpdated(_)
                | Event::ContactDeleted(_) => {
                    conn.send(net::Message::FetchContacts);
                }
                _ => (),
            },
        }

        None
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("Contacts");

        let search_contact = text_input("Search", &self.search_contact_input)
            .on_input(Message::SearchContactInputChange)
            .style(style::TextInput::ChatSearch)
            .width(SEARCH_CONTACT_WIDTH);
        let add_contact_btn = button(
            row![text("Add").size(18), plus_icon().size(14)]
                .align_items(alignment::Alignment::Center)
                .spacing(2),
        )
        .padding(5)
        .on_press(Message::OpenAddContactModal);
        let import_btn = button(import_icon().size(18))
            .padding(5)
            .on_press(Message::OpenImportContactModal);
        let send_btn = button(to_cloud_icon().size(18))
            .padding(5)
            .on_press(Message::OpenSendContactModal);

        let utils_row = row![
            search_contact,
            Space::with_width(Length::Fill),
            add_contact_btn,
            import_btn,
            send_btn
        ]
        .spacing(5)
        .width(Length::Fill);

        let contact_list: Element<_> = self
            .contacts
            .iter()
            .filter(|c| contact_matches_search(c, &self.search_contact_input))
            .map(|c| ContactRow::from_db_contact(c))
            .fold(column![].spacing(5), |col, contact| {
                col.push(contact.view().map(Message::ContactRowMessage))
            })
            .into();
        let contact_list_scroller = column![ContactRow::header(), common_scrollable(contact_list)];
        let content: Element<_> = column![title, utils_row, contact_list_scroller]
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        container(content).center_x().center_y().into()
    }
}

const SEARCH_CONTACT_WIDTH: f32 = 200.0;
