use super::Theme;
use iced::widget::slider;
use iced_style::{
    slider::{Handle, HandleShape, Rail},
    Color,
};

#[derive(Debug, Clone, Copy, Default)]
pub enum Slider {
    #[default]
    Default,
}

impl slider::StyleSheet for Theme {
    type Style = Slider;

    fn active(&self, _style: &Self::Style) -> iced_style::slider::Appearance {
        iced_style::slider::Appearance {
            rail: Rail {
                colors: (Color::WHITE, Color::BLACK),
                width: 2.0,
            },
            handle: Handle {
                shape: HandleShape::Circle { radius: 10.0 },
                color: Color::from_rgb8(120, 120, 120),
                border_width: 1.0,
                border_color: Color::WHITE,
            },
        }
    }

    fn hovered(&self, _style: &Self::Style) -> iced_style::slider::Appearance {
        iced_style::slider::Appearance {
            rail: Rail {
                colors: (Color::WHITE, Color::BLACK),
                width: 2.0,
            },
            handle: Handle {
                shape: HandleShape::Circle { radius: 10.0 },
                color: Color::from_rgb8(120, 120, 120),
                border_width: 1.0,
                border_color: Color::WHITE,
            },
        }
    }

    fn dragging(&self, _style: &Self::Style) -> iced_style::slider::Appearance {
        iced_style::slider::Appearance {
            rail: Rail {
                colors: (Color::WHITE, Color::BLACK),
                width: 2.0,
            },
            handle: Handle {
                shape: HandleShape::Circle { radius: 10.0 },
                color: Color::from_rgb8(120, 120, 120),
                border_width: 1.0,
                border_color: Color::WHITE,
            },
        }
    }
}
