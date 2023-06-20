use crate::components::{async_file_importer, card, AsyncFileImporter};
use crate::db::DbContact;
use crate::error::BackendClosed;
use crate::net::{self, BackEndConnection, BackendEvent};
use crate::style;
use crate::widget::Element;
use crate::{types::UncheckedEvent, utils::json_reader};
use iced::alignment;
use iced::widget::{button, column, row, text};
use iced::Command;
use iced::Length;
use iced_aw::Modal;
use nostr::Tag;
use std::fmt::Debug;
use std::path::Path;

use super::ModalView;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
    FileImporterMessage(async_file_importer::Message),
    SaveImportedContacts(Vec<DbContact>),
}

pub struct ImportContactList<M: Clone + Debug> {
    pub imported_contacts: Vec<DbContact>,
    pub file_importer: AsyncFileImporter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Clone + Debug> ImportContactList<M> {
    pub fn new() -> Self {
        Self {
            imported_contacts: vec![],
            file_importer: AsyncFileImporter::new("/path/to/contacts.json")
                .file_filter("JSON File", &["json"]),
            phantom: std::marker::PhantomData,
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
}

impl<M: Clone + Debug + 'static + Send> ModalView for ImportContactList<M> {
    type UnderlayMessage = M;
    type Message = CMessage<M>;

    fn backend_event(
        &mut self,
        event: BackendEvent,
        _conn: &mut BackEndConnection,
    ) -> Result<(), BackendClosed> {
        if let BackendEvent::RFDPickedFile(path) = event {
            self.handle_file_importer_message(&path);
            self.file_importer
                .update(async_file_importer::Message::UpdateFilePath(path), _conn)?;
        }
        Ok(())
    }

    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> Result<(Command<Self::Message>, bool), BackendClosed> {
        let command = Command::none();
        match message {
            CMessage::UnderlayMessage(_) => (),
            CMessage::CloseModal => return Ok((command, true)),
            CMessage::SaveImportedContacts(imported_contacts) => {
                // TODO: two buttons, one replace. other merge
                let is_replace = true;
                conn.send(net::ToBackend::ImportContacts(
                    imported_contacts,
                    is_replace,
                ))?;
                return Ok((command, true));
            }
            CMessage::FileImporterMessage(msg) => self.file_importer.update(msg, conn)?,
        }

        Ok((command, false))
    }

    fn view<'a>(
        &'a self,
        underlay: impl Into<Element<'a, Self::UnderlayMessage>>,
    ) -> Element<'a, Self::Message> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);

        Modal::new(true, underlay_component, || {
            let importer_cp = self.file_importer.view().map(CMessage::FileImporterMessage);
            let found_contacts_txt = match self.imported_contacts.len() {
                0 => text(""),
                n => text(format!("Found contacts: {}", n)),
            };
            let stats_row = row![found_contacts_txt];

            let card_body = column![importer_cp, stats_row].spacing(4).padding(20);
            let card_footer = row![
                button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center),)
                    .style(style::Button::Bordered)
                    .width(Length::Fill)
                    .on_press(CMessage::CloseModal),
                button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                    .width(Length::Fill)
                    .on_press(CMessage::SaveImportedContacts(
                        self.imported_contacts.clone()
                    ))
            ]
            .spacing(10)
            .width(Length::Fill);

            card(card_body, card_footer).max_width(MODAL_WIDTH).into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }
}

const MODAL_WIDTH: f32 = 400.0;
