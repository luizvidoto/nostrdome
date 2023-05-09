#![allow(dead_code)]

use iced::{alignment, widget::text, Font};

use crate::widget::Text;

fn solid_icon(unicode: char) -> Text<'static> {
    text(unicode.to_string())
        .font(SOLID_ICONS)
        .width(20)
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

fn regular_icon(unicode: char) -> Text<'static> {
    text(unicode.to_string())
        .font(REGULAR_ICONS)
        .width(20)
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

pub fn refresh_icon() -> Text<'static> {
    solid_icon('\u{F021}')
}

pub fn retweet_icon() -> Text<'static> {
    solid_icon('\u{F079}')
}
pub fn circle_up_icon() -> Text<'static> {
    solid_icon('\u{F35B}')
}
pub fn file_icon_solid() -> Text<'static> {
    solid_icon('\u{F15B}')
}
pub fn file_icon_regular() -> Text<'static> {
    regular_icon('\u{F15B}')
}

pub fn edit_icon() -> Text<'static> {
    solid_icon('\u{F303}')
}

pub fn satellite_icon() -> Text<'static> {
    solid_icon('\u{F7c0}')
}

pub fn signal_icon() -> Text<'static> {
    solid_icon('\u{F012}')
}

pub fn server_icon() -> Text<'static> {
    solid_icon('\u{F233}')
}

pub fn solid_circle_icon() -> Text<'static> {
    solid_icon('\u{F111}')
}

pub fn regular_circle_icon() -> Text<'static> {
    regular_icon('\u{F111}')
}

pub fn menu_bars_icon() -> Text<'static> {
    solid_icon('\u{F0C9}')
}

pub fn settings_icon() -> Text<'static> {
    solid_icon('\u{F013}')
}

pub fn delete_icon() -> Text<'static> {
    solid_icon('\u{F2ED}')
}

pub fn send_icon() -> Text<'static> {
    solid_icon('\u{F1D8}')
}

pub fn check_icon() -> Text<'static> {
    solid_icon('\u{F00C}')
}

pub fn double_check_icon() -> Text<'static> {
    solid_icon('\u{F560}')
}

pub fn circle_check_icon() -> Text<'static> {
    solid_icon('\u{F058}')
}

pub fn circle_xmark_icon() -> Text<'static> {
    solid_icon('\u{F057}')
}

pub fn xmark_icon() -> Text<'static> {
    solid_icon('\u{F00D}')
}

pub fn to_cloud_icon() -> Text<'static> {
    solid_icon('\u{F0EE}')
}

pub fn download_icon() -> Text<'static> {
    solid_icon('\u{F019}')
}

pub fn import_icon() -> Text<'static> {
    solid_icon('\u{F56F}')
}

pub fn triangle_warn_icon() -> Text<'static> {
    solid_icon('\u{F071}')
}

pub fn add_friend_icon() -> Text<'static> {
    solid_icon('\u{F234}')
}

pub fn plus_icon() -> Text<'static> {
    solid_icon('\u{002b}')
}

// Fonts
const SOLID_ICONS: Font = Font::External {
    name: "FA_Solid_Icons",
    bytes: include_bytes!("../fonts/fa_solid.otf"),
};

const REGULAR_ICONS: Font = Font::External {
    name: "FA_Icons",
    bytes: include_bytes!("../fonts/fa_icons.otf"),
};
