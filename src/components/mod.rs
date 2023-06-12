use iced::widget::column;

pub mod async_file_importer;
pub mod chat_contact;
pub mod chat_list_container;
pub mod contact_list;
pub mod contact_row;
mod copy_btn;
mod custom_widgets;
pub mod relay_row;
mod scrollables;
pub mod status_bar;
pub mod text;
pub mod text_input_group;

use crate::style;
use crate::widget::Element;
pub use async_file_importer::AsyncFileImporter;
pub use contact_row::ContactRow;
pub use copy_btn::copy_btn;
pub use custom_widgets::{floating_element, FloatingElement, MouseArea, Responsive};
use iced::widget::container;
use iced::Length;
pub use relay_row::RelayRow;
pub use scrollables::{common_scrollable, invisible_scrollable};
pub use status_bar::StatusBar;
use text::title;

pub fn inform_card<'a, Message: 'a>(
    title_text: &str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = column![title(title_text).width(Length::Shrink), content.into()].spacing(10);
    let inner = container(content)
        .padding(30)
        .style(style::Container::ContactList);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(style::Container::ChatContainer)
        .into()
}
