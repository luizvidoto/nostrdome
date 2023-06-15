use crate::{components::text::title, style, widget::Element};
use iced::widget::{column, container, row, text};
use iced::Color;

use super::route::Route;

#[derive(Debug, Clone)]
pub enum Message {}

pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }
}
impl Route for State {
    type Message = Message;
    fn view(&self, _selected_theme: Option<style::Theme>) -> Element<'_, Self::Message> {
        let page_title = title("Color Palettes");

        let color_grid = style::Theme::ALL
            .into_iter()
            .map(|t| t.palette())
            .enumerate()
            .fold(row![title_column()].spacing(5), |row, (idx, palette)| {
                row.push(make_palette_col(idx.to_string(), palette))
            });
        let ids_to_name = style::Theme::ALL
            .into_iter()
            .enumerate()
            .fold(column![].spacing(5), |col, (idx, t)| {
                col.push(text(format!("[{}]: {}", idx, t)))
            });
        column![page_title, color_grid, ids_to_name]
            .spacing(10)
            .into()
    }
}
fn title_column() -> Element<'static, Message> {
    column![
        text("").height(ROW_HEIGHT),
        // BASE
        text("base.background").height(ROW_HEIGHT),
        text("base.foreground").height(ROW_HEIGHT),
        text("base.text").height(ROW_HEIGHT),
        text("base.comment").height(ROW_HEIGHT),
        // NORMAL
        text("normal.primary").height(ROW_HEIGHT),
        text("normal.primary_lighter").height(ROW_HEIGHT),
        text("normal.secondary").height(ROW_HEIGHT),
        text("normal.error").height(ROW_HEIGHT),
        text("normal.success").height(ROW_HEIGHT),
    ]
    .spacing(2)
    .into()
}
fn make_palette_col<S>(name: S, palette: style::ColorPalette) -> Element<'static, Message>
where
    S: Into<String>,
{
    column![
        text(name.into()).height(ROW_HEIGHT),
        // BASE
        colored_container(palette.base.background),
        colored_container(palette.base.foreground),
        colored_container(palette.base.text),
        colored_container(palette.base.comment),
        // NORMAL
        colored_container(palette.normal.primary),
        colored_container(palette.normal.primary_variant),
        colored_container(palette.normal.secondary),
        colored_container(palette.normal.error),
        colored_container(palette.normal.success),
    ]
    .spacing(2)
    .into()
}

fn colored_container(color: Color) -> Element<'static, Message> {
    container(text(""))
        .height(ROW_HEIGHT)
        .width(COLOR_COL_WIDTH)
        .style(style::Container::WithColor(color))
        .into()
}

const ROW_HEIGHT: u16 = 20;
const COLOR_COL_WIDTH: u16 = 20;
