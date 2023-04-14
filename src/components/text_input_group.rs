use iced::{
    widget::{column, container, row, text, text_input, tooltip},
    Color, Element, Theme,
};
use iced_style::container::StyleSheet;

pub fn text_input_group<'a, Message: Clone + 'a>(
    label_str: &str,
    placeholder: &str,
    value: &str,
    tooltip_str: Option<String>,
    on_change: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let label = text(label_str);
    let tooltip: Element<_> = if let Some(tooltip_str) = tooltip_str {
        let tooltip_icon = container(text("?").size(15))
            .style(iced::theme::Container::Custom(Box::new(
                TooltipContainer {},
            )))
            .padding([2, 4]);
        tooltip(tooltip_icon, tooltip_str, tooltip::Position::Top).into()
    } else {
        text("").into()
    };
    let label_row = row![label, tooltip].spacing(4);
    let txt_input = text_input(placeholder, value).on_input(on_change);

    column![label_row, txt_input].into()
}

struct TooltipContainer {}
impl StyleSheet for TooltipContainer {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Color::from_rgb8(150, 150, 150).into()),
            border_radius: 10.0,
            ..Default::default()
        }
    }
}
