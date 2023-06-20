use iced::widget::button;
use iced::{Color, Vector};
use iced_style::Background;

use crate::utils::{change_color_by_type, darken_color, lighten_color};

use super::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub enum Button {
    #[default]
    Primary,
    Danger,
    Invisible,
    ContactCard,
    ActiveContactCard,
    ActiveMenuBtn,
    MenuBtn,
    StatusBarButton,
    Bordered,
    Notification,
    ContextMenuButton,
    Link,
    HighlightButton,
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let theme_type = self.theme_meta().theme_type;
        let appearance = button::Appearance {
            border_width: 1.0,
            border_radius: 8.0,
            ..button::Appearance::default()
        };

        let contact_card = button::Appearance {
            background: self.palette().base.foreground.into(),
            text_color: self.palette().base.text,
            border_radius: 8.0,
            border_width: 0.0,
            ..appearance
        };
        let menu = button::Appearance {
            background: self.palette().base.foreground.into(),
            text_color: self.palette().base.text,
            border_width: 0.0,
            ..appearance
        };

        match style {
            Button::Primary => button::Appearance {
                background: self.palette().normal.primary.into(),
                text_color: self.palette().base.text,
                ..appearance
            },
            Button::Danger => button::Appearance {
                background: self.palette().normal.error.into(),
                text_color: self.palette().base.text,
                ..appearance
            },
            Button::Invisible => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().base.text,
                ..appearance
            },
            Button::Bordered => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: self.palette().normal.primary,
                text_color: self.palette().base.text,
                border_width: 2.0,
                ..appearance
            },
            Button::HighlightButton => button::Appearance {
                background: darken_color(self.palette().normal.primary, 0.1).into(),
                border_color: Color::WHITE,
                text_color: Color::WHITE,
                border_radius: 4.0,
                border_width: 1.5,
                ..appearance
            },
            Button::ContactCard => contact_card,
            Button::ActiveContactCard => button::Appearance {
                background: (change_color_by_type(theme_type, self.palette().base.foreground, 0.1))
                    .into(),
                ..contact_card
            },

            Button::MenuBtn => menu,
            Button::ActiveMenuBtn => button::Appearance {
                background: self.palette().normal.primary.into(),
                ..menu
            },

            Button::StatusBarButton => button::Appearance {
                background: self.palette().base.foreground.into(),
                text_color: self.palette().base.text,
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                ..appearance
            },
            Button::Notification => button::Appearance {
                background: self.palette().normal.primary.into(),
                text_color: self.palette().base.text,
                border_color: Color::TRANSPARENT,
                border_radius: 20.0,
                border_width: 0.0,
                ..appearance
            },
            Button::ContextMenuButton => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().base.text,
                border_color: Color::TRANSPARENT,
                border_radius: 5.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
            Button::Link => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: Color::from_rgb8(0x00, 0x7a, 0xff),
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        let theme_type = self.theme_meta().theme_type;

        let background = active.background.map(|background| match background {
            Background::Color(color) => {
                let color = change_color_by_type(theme_type, color, 0.05);
                Background::Color(color)
            }
        });
        let border_color = {
            let color = change_color_by_type(theme_type, active.border_color, 0.05);
            Color { a: 1.0, ..color }
        };
        let text_color = {
            let color = change_color_by_type(theme_type, active.text_color, 0.05);
            Color { a: 1.0, ..color }
        };

        let changed = button::Appearance {
            background,
            text_color,
            ..active
        };

        match style {
            Button::Primary => changed,
            Button::Danger => changed,
            Button::Invisible => self.active(style),
            Button::Bordered => button::Appearance {
                border_color,
                text_color,
                ..self.active(style)
            },
            Button::HighlightButton => button::Appearance {
                background: self.palette().normal.primary.into(),
                ..self.active(style)
            },
            Button::ContactCard => changed,
            Button::ActiveContactCard => changed,
            Button::MenuBtn => changed,
            Button::ActiveMenuBtn => changed,
            Button::StatusBarButton => changed,
            Button::Notification => self.active(style),
            Button::ContextMenuButton => button::Appearance {
                background: change_color_by_type(theme_type, self.palette().base.background, 0.05)
                    .into(),
                ..self.active(style)
            },
            Button::Link => self.active(style),
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let hovered = self.hovered(style);
        let theme_type = self.theme_meta().theme_type;

        let background = hovered.background.map(|background| match background {
            Background::Color(color) => {
                let color = change_color_by_type(theme_type, color, 0.05);
                Background::Color(color)
            }
        });

        let changed = button::Appearance {
            background,
            ..hovered
        };

        match style {
            Button::Primary => changed,
            Button::Danger => changed,
            Button::Invisible => self.active(style),
            Button::Bordered => self.active(style),
            Button::ContactCard => changed,
            Button::ActiveContactCard => changed,
            Button::MenuBtn => changed,
            Button::HighlightButton => changed,
            Button::ActiveMenuBtn => changed,
            Button::StatusBarButton => self.active(style),
            Button::Notification => self.active(style),
            Button::ContextMenuButton => self.active(style),
            Button::Link => self.active(style),
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        let def = button::Appearance {
            shadow_offset: Vector::default(),
            background: active.background.map(|background| match background {
                Background::Color(color) => Background::Color(Color {
                    a: color.a * 0.5,
                    ..color
                }),
            }),
            text_color: Color {
                a: active.text_color.a * 0.5,
                ..active.text_color
            },
            ..active
        };

        match style {
            Button::Primary => def,
            Button::Danger => def,
            Button::Invisible => def,
            Button::Bordered => def,
            Button::ContactCard => def,
            Button::ActiveContactCard => def,
            Button::HighlightButton => def,
            Button::ActiveMenuBtn => def,
            Button::MenuBtn => def,
            Button::StatusBarButton => def,
            Button::Link => def,
            Button::ContextMenuButton => self.active(style),
            Button::Notification => self.active(style),
        }
    }
}
