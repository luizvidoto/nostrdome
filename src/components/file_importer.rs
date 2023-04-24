use iced::{
    widget::{button, row, text_input},
    Length,
};
use rfd::FileDialog;

use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ChooseFile,
    OnChoose(String),
}
#[derive(Debug, Clone)]
pub struct FileFilter {
    name: String,
    extensions: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct FileImporter {
    placeholder: String,
    file_input: String,
    file_filter: Option<FileFilter>,
}
impl FileImporter {
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
        let file_input = text_input(&self.placeholder, &self.file_input).width(Length::Fill);
        let file_input_btn = button("...")
            .width(Length::Fixed(30.0))
            .on_press(Message::ChooseFile);
        row![file_input, file_input_btn]
            .spacing(10)
            .width(Length::Fill)
            .into()
    }
    pub fn reset_inputs(&mut self) {
        self.file_input = "".into();
    }
    pub fn update(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::ChooseFile => {
                let mut rfd_instance = FileDialog::new().set_directory("/");
                if let Some(filter) = &self.file_filter {
                    rfd_instance = rfd_instance.add_filter(
                        &filter.name,
                        &filter
                            .extensions
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<_>>(),
                    );
                }
                if let Some(path) = rfd_instance.pick_file() {
                    if let Some(p) = path.to_str() {
                        self.file_input = p.to_owned();
                        return Some(Message::OnChoose(self.file_input.clone()));
                    }
                }
            }
            _ => (),
        }
        None
    }
}
