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
    icon('\u{F303}')
}

pub fn satellite_icon() -> Text<'static> {
    icon('\u{F7c0}')
}

pub fn signal_icon() -> Text<'static> {
    icon('\u{F012}')
}

pub fn server_icon() -> Text<'static> {
    icon('\u{F233}')
}

pub fn circle_icon() -> Text<'static> {
    icon('\u{F111}')
}

pub fn menu_bars_icon() -> Text<'static> {
    icon('\u{F0C9}')
}

pub fn settings_icon() -> Text<'static> {
    icon('\u{F013}')
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

pub fn double_check_icon() -> Text<'static> {
    icon('\u{F560}')
}

pub fn circle_check_icon() -> Text<'static> {
    icon('\u{F058}')
}

pub fn circle_xmark_icon() -> Text<'static> {
    icon('\u{F057}')
}

pub fn xmark_icon() -> Text<'static> {
    icon('\u{F00D}')
}

pub fn to_cloud_icon() -> Text<'static> {
    icon('\u{F0EE}')
}

pub fn import_icon() -> Text<'static> {
    icon('\u{F56F}')
}

pub fn triangle_warn_icon() -> Text<'static> {
    icon('\u{F071}')
}

pub fn add_friend_icon() -> Text<'static> {
    icon('\u{F234}')
}

pub fn plus_icon() -> Text<'static> {
    icon('\u{002b}')
}

// Fonts
const ICONS: Font = Font::External {
    name: "FA_Icons",
    bytes: include_bytes!("../fonts/fa_solid.otf"),
};
