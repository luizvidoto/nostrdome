use iced::widget::{button, column, container, row, scrollable, text_input, Space};
use iced::Length;

use crate::components::{contact_row, ContactRow};
use crate::net::BackEndConnection;
use crate::style;
use crate::utils::contact_matches_search;
use crate::widget::Element;
use crate::{components::text::title, db::DbContact, net};

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(net::Event),
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
    pub fn new(back_conn: &mut BackEndConnection) -> Self {
        back_conn.send(net::Message::FetchContacts);
        Self {
            contacts: vec![],
            search_contact_input: "".into(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        back_conn: &mut BackEndConnection,
    ) -> Option<Message> {
        match message {
            Message::OpenSendContactModal => (),
            Message::OpenEditContactModal(_) => (),
            Message::OpenAddContactModal => (),
            Message::OpenImportContactModal => (),
            Message::SearchContactInputChange(text) => self.search_contact_input = text,
            Message::ContactRowMessage(ct_msg) => match ct_msg {
                contact_row::Message::DeleteContact(contact) => {
                    back_conn.send(net::Message::DeleteContact(contact))
                }
                contact_row::Message::EditContact(contact) => {
                    // self.modal_state = ModalState::add_contact(Some(contact));
                    return Some(Message::OpenEditContactModal(contact));
                }
            },
            Message::DeleteContact(contact) => {
                back_conn.send(net::Message::DeleteContact(contact));
            }
            Message::BackEndEvent(db_ev) => match db_ev {
                net::Event::GotContacts(db_contacts) => {
                    self.contacts = db_contacts;
                }
                net::Event::DBSuccessEvent(kind) => match kind {
                    net::SuccessKind::ContactCreated(_)
                    | net::SuccessKind::ContactDeleted(_)
                    | net::SuccessKind::ContactUpdated(_)
                    | net::SuccessKind::ContactsImported(_) => {
                        back_conn.send(net::Message::FetchContacts);
                    }
                    _ => (),
                },
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
        let add_contact_btn = button("Add").on_press(Message::OpenAddContactModal);
        let import_btn = button("Import").on_press(Message::OpenImportContactModal);
        let send_btn = button("Send").on_press(Message::OpenSendContactModal);

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
        let contact_list_scroller = column![ContactRow::header(), scrollable(contact_list)];
        let content: Element<_> = column![title, utils_row, contact_list_scroller]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        container(content).center_x().center_y().into()
    }
}

const SEARCH_CONTACT_WIDTH: f32 = 200.0;
