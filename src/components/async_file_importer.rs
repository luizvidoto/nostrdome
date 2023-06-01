use std::path::PathBuf;

use iced::{
    widget::{button, row, text_input},
    Length,
};

use crate::{
    net::{BackEndConnection, ToBackend},
    widget::Element,
};

#[derive(Debug, Clone)]
pub enum Message {
    ChooseFile,
    UpdateFilePath(PathBuf),
}
#[derive(Debug, Clone)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct AsyncFileImporter {
    placeholder: String,
    file_input: PathBuf,
    file_filter: Option<FileFilter>,
}
impl AsyncFileImporter {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            file_input: "".into(),
            file_filter: None,
        }
    }
    pub fn file_filter(self, name: &str, extensions: &[&str]) -> Self {
        Self {
            file_filter: Some(FileFilter {
                name: name.to_owned(),
                extensions: extensions.iter().map(|ext| ext.to_string()).collect(),
            }),
            ..self
        }
    }
    pub fn view(&self) -> Element<'static, Message> {
        let file_input =
            text_input(&self.placeholder, &self.file_input.to_string_lossy()).width(Length::Fill);
        let file_input_btn = button("...").width(30).on_press(Message::ChooseFile);
        row![file_input, file_input_btn]
            .spacing(10)
            .width(Length::Fill)
            .into()
    }
    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::ChooseFile => conn.send(ToBackend::ChooseFile(self.file_filter.clone())),
            Message::UpdateFilePath(path) => {
                self.file_input = path;
            }
        }
    }
}
