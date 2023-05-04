use super::Theme;
use iced::widget::scrollable;
use iced::Color;

#[derive(Debug, Clone, Copy, Default)]
pub enum Scrollable {
    #[default]
    Default,
}

const SCROLLABLE_BORDER_RADIUS: f32 = 4.0;
impl scrollable::StyleSheet for Theme {
    type Style = Scrollable;

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Color::TRANSPARENT.into(),
            border_radius: SCROLLABLE_BORDER_RADIUS,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: Color::TRANSPARENT,
                border_radius: SCROLLABLE_BORDER_RADIUS,
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }
    fn dragging(&self, style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            scroller: scrollable::Scroller {
                color: self.pallete().text_color,
                ..self.hovered(style, true).scroller
            },
            ..self.hovered(style, true)
        }
    }
    fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> scrollable::Scrollbar {
        if is_mouse_over_scrollbar {
            scrollable::Scrollbar {
                background: self.pallete().hovered_bg_scrollbar_mo.into(),
                scroller: scrollable::Scroller {
                    color: self.pallete().hovered_bg_scroller_mo,
                    ..self.active(style).scroller
                },
                ..self.active(style)
            }
        } else {
            scrollable::Scrollbar {
                background: self.pallete().hovered_bg_scrollbar.into(),
                scroller: scrollable::Scroller {
                    color: self.pallete().hovered_bg_scroller,
                    ..self.active(style).scroller
                },
                ..self.active(style)
            }
        }
    }
}
