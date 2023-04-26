use iced::{alignment, widget::text, Font};

use crate::widget::Text;

fn icon(unicode: char) -> Text<'static> {
    text(unicode.to_string())
        .font(ICONS)
        .width(20)
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

pub fn edit_icon() -> Text<'static> {
    icon('\u{F044}')
}

pub fn delete_icon() -> Text<'static> {
    icon('\u{F2ED}')
}

pub fn send_icon() -> Text<'static> {
    icon('\u{F1D8}')
}

pub fn check_icon() -> Text<'static> {
    icon('\u{F00C}')
}

pub fn circle_xmark_icon() -> Text<'static> {
    icon('\u{F057}')
}

pub fn xmark_icon() -> Text<'static> {
    icon('\u{F00D}')
}

// Fonts
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../fonts/fa_icons.otf"),
};
