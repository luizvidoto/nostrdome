use iced::widget::{column, container, radio, row, scrollable, scrollable::Properties, text};
use iced::Alignment;

use crate::{
    components::text::title,
    style::{self},
    widget::Element,
};

#[derive(Debug, Clone)]
pub enum Message {
    ChangeTheme(style::Theme),
}
pub fn view(selected_theme: Option<style::Theme>) -> Element<'static, Message> {
    let title = title("Appearance");
    let light_themes =
        style::Theme::LIGHT
            .into_iter()
            .fold(row![].padding([20, 0]).spacing(5), |row, t| {
                row.push(vertical_radio(
                    t.to_string(),
                    t,
                    selected_theme,
                    Message::ChangeTheme,
                ))
            });
    let light_themes = scrollable(light_themes).horizontal_scroll(Properties::default());
    let light_themes = column![text("Light Themes").size(24), light_themes].spacing(10);

    let dark_themes =
        style::Theme::DARK
            .into_iter()
            .fold(row![].padding([20, 0]).spacing(5), |row, t| {
                row.push(vertical_radio(
                    t.to_string(),
                    t,
                    selected_theme,
                    Message::ChangeTheme,
                ))
            });
    let dark_themes = scrollable(dark_themes).horizontal_scroll(Properties::default());
    let dark_themes = column![text("Dark Themes").size(24), dark_themes].spacing(10);

    column![title, light_themes, dark_themes].spacing(20).into()
}

fn vertical_radio<V, Message: 'static>(
    label: impl Into<String>,
    value: V,
    selected: Option<V>,
    on_click: impl FnOnce(V) -> Message,
) -> Element<'static, Message>
where
    Message: Clone,
    V: Copy + Eq,
{
    container(
        column![
            radio("", value, selected, on_click).spacing(0),
            text(label.into()),
        ]
        .align_items(Alignment::Center)
        .spacing(2),
    )
    .center_x()
    .center_y()
    .width(RADIO_WIDTH)
    .into()
}

const RADIO_WIDTH: u16 = 100;
