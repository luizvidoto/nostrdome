use iced::widget::button;
use iced::{Color, Vector};
use iced_style::Background;

use crate::utils::{darken_color, lighten_color};

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
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let primary = button::Appearance {
            background: self.palette().primary.into(),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
            shadow_offset: Vector { x: 0., y: 0. },
        };
        match style {
            Button::Primary => primary,
            Button::Danger => button::Appearance {
                background: self.palette().danger.into(),
                ..primary
            },
            Button::Invisible => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().text_color,
                ..primary
            },
            Button::Bordered => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: self.palette().primary,
                text_color: self.palette().text_color,
                border_radius: 8.0,
                border_width: 2.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
            Button::ContactCard => button::Appearance {
                background: self.palette().background.into(),
                text_color: self.palette().text_color,
                border_radius: 0.0,
                ..primary
            },
            Button::ActiveContactCard => button::Appearance {
                border_radius: 0.0,
                background: darken_color(self.palette().primary, 0.2).into(),
                text_color: self.palette().text_color,
                ..primary
            },
            Button::ActiveMenuBtn => button::Appearance {
                background: darken_color(self.palette().primary, 0.2).into(),
                text_color: self.palette().text_color,
                border_radius: 8.0,
                ..primary
            },
            Button::MenuBtn => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().text_color,
                border_radius: 8.0,
                ..primary
            },
            Button::StatusBarButton => button::Appearance {
                background: self.palette().status_bar_bg.into(),
                text_color: self.palette().status_bar_text_color,
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                ..primary
            },
            Button::Notification => button::Appearance {
                background: self.palette().notification.into(),
                text_color: self.palette().text_color,
                border_color: Color::TRANSPARENT,
                border_radius: 20.0,
                border_width: 0.0,
                ..primary
            },
            Button::ContextMenuButton => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().text_color,
                border_color: Color::TRANSPARENT,
                border_radius: 5.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
            Button::Link => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().link_color,
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        let background = active.background.map(|background| match background {
            Background::Color(color) => {
                let color = lighten_color(color, 0.1);
                Background::Color(color)
            }
        });
        let border_color = {
            let color = lighten_color(active.border_color, 0.1);
            Color { a: 1.0, ..color }
        };
        let text_color = Color {
            a: 1.0,
            ..active.text_color
        };
        let lighter = button::Appearance {
            shadow_offset: Vector::default(),
            background,
            text_color,
            border_color: border_color.clone(),
            ..active
        };

        match style {
            Button::Primary => lighter,
            Button::Danger => lighter,
            Button::Invisible => self.active(style),
            Button::Bordered => button::Appearance {
                border_color,
                text_color,
                ..self.active(style)
            },
            Button::ContactCard => lighter,
            Button::ActiveContactCard => lighter,
            Button::MenuBtn => lighter,
            Button::ActiveMenuBtn => lighter,
            Button::StatusBarButton => lighter,
            Button::Notification => self.active(style),
            Button::ContextMenuButton => button::Appearance {
                background: lighten_color(self.palette().background, 0.1).into(),
                ..self.active(style)
            },
            Button::Link => self.active(style),
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => self.active(style),
            Button::Danger => self.active(style),
            Button::Invisible => self.active(style),
            Button::Bordered => self.active(style),
            Button::ContactCard => self.active(style),
            Button::ActiveContactCard => self.active(style),
            Button::ActiveMenuBtn => self.active(style),
            Button::MenuBtn => self.active(style),
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
            Button::ActiveMenuBtn => def,
            Button::MenuBtn => def,
            Button::StatusBarButton => def,
            Button::Link => def,
            Button::ContextMenuButton => self.active(style),
            Button::Notification => self.active(style),
        }
    }
}
