use crate::components::{async_file_importer, AsyncFileImporter};
use crate::db::DbContact;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::widget::Element;
use crate::{types::UncheckedEvent, utils::json_reader};
use iced::alignment;
use iced::widget::{button, column, row, text};
use iced::Length;
use iced_aw::{Card, Modal};
use nostr::Tag;
use std::fmt::Debug;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
    FileImporterMessage(async_file_importer::Message),
    SaveImportedContacts(Vec<DbContact>),
}

pub struct ImportContactList {
    pub imported_contacts: Vec<DbContact>,
    pub file_importer: AsyncFileImporter,
}
impl ImportContactList {
    pub fn new() -> Self {
        Self {
            imported_contacts: vec![],
            file_importer: AsyncFileImporter::new("/path/to/contacts.json")
                .file_filter("JSON File", &["json"]),
        }
    }
    pub fn update<'a, M: 'a + Clone + Debug>(
        &'a mut self,
        message: CMessage<M>,
        conn: &mut BackEndConnection,
    ) -> bool {
        match message {
            CMessage::UnderlayMessage(_) => (),
            CMessage::CloseModal => return true,
            CMessage::SaveImportedContacts(imported_contacts) => {
                // TODO: two buttons, one replace. other merge
                let is_replace = true;
                conn.send(net::ToBackend::ImportContacts(
                    imported_contacts,
                    is_replace,
                ));
                // *self = ModalState::Off;
                return true;
            }
            CMessage::FileImporterMessage(msg) => self.file_importer.update(msg, conn),
        }
        false
    }
    pub fn backend_event(&mut self, event: BackendEvent, _conn: &mut BackEndConnection) {
        match event {
            BackendEvent::RFDPickedFile(path) => {
                self.handle_file_importer_message(&path);
                self.file_importer
                    .update(async_file_importer::Message::UpdateFilePath(path), _conn);
            }
            _ => (),
        }
    }
    fn handle_file_importer_message<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        match json_reader::<P, UncheckedEvent>(path) {
            Ok(contact_event) => {
                if let nostr::event::Kind::ContactList = contact_event.kind {
                    self.update_imported_contacts(&contact_event.tags);
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
    fn update_imported_contacts(&mut self, tags: &[Tag]) {
        let (oks, errs): (Vec<_>, Vec<_>) = tags
            .iter()
            .map(DbContact::from_tag)
            .partition(Result::is_ok);

        let errors: Vec<_> = errs.into_iter().map(Result::unwrap_err).collect();

        for e in errors {
            tracing::error!("{}", e);
        }

        self.imported_contacts = oks.into_iter().map(Result::unwrap).collect();
    }

    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, || {
            let importer_cp = self.file_importer.view().map(CMessage::FileImporterMessage);
            let found_contacts_txt = match self.imported_contacts.len() {
                0 => text(""),
                n => text(format!("Found contacts: {}", n)),
            };
            let stats_row = row![found_contacts_txt];

            let card_header = text("Import Contacts");
            let card_body = column![importer_cp, stats_row].spacing(4);
            let card_footer = row![
                button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center),)
                    .width(Length::Fill)
                    .on_press(CMessage::CloseModal),
                button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                    .width(Length::Fill)
                    .on_press(CMessage::SaveImportedContacts(
                        self.imported_contacts.clone()
                    ))
            ]
            .spacing(10)
            .padding(5)
            .width(Length::Fill);

            let card: Element<_> = Card::new(card_header, card_body)
                .foot(card_footer)
                .max_width(MODAL_WIDTH)
                .on_close(CMessage::CloseModal)
                .style(crate::style::Card::Default)
                .into();

            card
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }
}

const MODAL_WIDTH: f32 = 400.0;
