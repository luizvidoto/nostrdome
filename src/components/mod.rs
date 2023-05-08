pub mod contact_card;
pub mod relay_row;
pub mod text;
pub mod text_input_group;
use iced::widget::column;
pub use relay_row::RelayRow;
pub mod contact_row;
pub use contact_row::ContactRow;
pub mod file_importer;
pub use file_importer::FileImporter;
mod common_scrollable;
pub use common_scrollable::common_scrollable;

pub mod status_bar;
pub use status_bar::StatusBar;

use crate::style;
use crate::widget::Element;
use iced::widget::container;
use iced::Length;
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
