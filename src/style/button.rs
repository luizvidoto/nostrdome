use iced::widget::button;
use iced::{Color, Vector};
use iced_style::Background;

use super::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub enum Button {
    #[default]
    Primary,
    Secondary,
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
            text_color: self.palette().text_color,
            shadow_offset: Vector { x: 0., y: 0. },
        };
        match style {
            Button::Primary => primary,
            Button::Secondary => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: self.palette().primary,
                text_color: self.palette().primary,
                ..primary
            },
            Button::Danger => button::Appearance {
                background: self.palette().danger.into(),
                ..primary
            },
            Button::Invisible => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: Color::TRANSPARENT,
                text_color: self.palette().text_color,
                ..primary
            },
            Button::Bordered => button::Appearance {
                background: Color::TRANSPARENT.into(),
                border_color: Color::from_rgb8(120, 120, 120),
                text_color: self.palette().primary,
                ..primary
            },
            Button::ContactCard => button::Appearance {
                background: self.palette().background.into(),
                text_color: self.palette().text_color,
                border_radius: 0.0,
                ..primary
            },
            Button::ActiveContactCard => button::Appearance {
                border_radius: 0.0,
                background: self.palette().primary_darker.into(),
                text_color: self.palette().text_color,
                ..primary
            },
            Button::ActiveMenuBtn => button::Appearance {
                background: self.palette().primary_darker.into(),
                text_color: self.palette().text_color,
                border_radius: 5.0,
                ..primary
            },
            Button::MenuBtn => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().text_color,
                border_radius: 5.0,
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
                text_color: Color::from_rgb8(180, 180, 180),
                border_color: Color::TRANSPARENT,
                border_radius: 5.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
            Button::Link => button::Appearance {
                background: Color::TRANSPARENT.into(),
                text_color: self.palette().primary_lighter,
                border_color: Color::TRANSPARENT,
                border_radius: 0.0,
                border_width: 0.0,
                shadow_offset: Vector { x: 0., y: 0. },
            },
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => button::Appearance {
                background: self.palette().primary_lighter.into(),
                ..self.active(style)
            },
            Button::Secondary => self.active(style),
            Button::Danger => self.active(style),
            Button::Invisible => self.active(style),
            Button::Bordered => button::Appearance {
                border_color: self.palette().primary,
                text_color: self.palette().primary,
                ..self.active(style)
            },
            Button::ContactCard => button::Appearance {
                background: self.palette().background.into(),
                text_color: self.palette().text_color,
                ..self.active(style)
            },

            Button::ActiveContactCard => self.active(style),
            Button::ActiveMenuBtn => button::Appearance {
                ..self.active(style)
            },
            Button::MenuBtn => button::Appearance {
                background: self.palette().background.into(),
                text_color: self.palette().text_color,
                ..self.active(style)
            },
            Button::StatusBarButton => button::Appearance {
                background: self.palette().hover_status_bar_bg.into(),
                ..self.active(style)
            },
            Button::Notification => self.active(style),
            Button::ContextMenuButton => button::Appearance {
                background: self.palette().primary.into(),
                text_color: self.palette().text_color,
                ..self.active(style)
            },
            Button::Link => self.active(style),
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => self.active(style),
            Button::Secondary => self.active(style),
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
            Button::Secondary => def,
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
