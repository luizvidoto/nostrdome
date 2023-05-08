// Always import widget types from this module since it
// uses our custom theme instead of the built-in iced::Theme.
// Otherwise you will get compilation errors since iced::Element
// expects use of iced::Theme by default.
#![allow(dead_code)]
use crate::style::Theme;

pub type Renderer = iced::Renderer<Theme>;
pub type Element<'a, Message> = iced::Element<'a, Message, Renderer>;
pub type Container<'a, Message> = iced::widget::Container<'a, Message, Renderer>;
pub type Scrollable<'a, Message> = iced::widget::Scrollable<'a, Message, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Renderer>;
pub type Column<'a, Message> = iced::widget::Column<'a, Message, Renderer>;

/// Creates a new [`Button`] with the provided content.
///
/// [`Button`]: widget::Button
pub fn button<'a, Message>(content: impl Into<Element<'a, Message>>) -> Button<'a, Message> {
    iced::widget::Button::new(content)
}
