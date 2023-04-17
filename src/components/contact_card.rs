use iced::widget::{button, column, row, text};
use iced::{Color, Element, Length};

#[derive(Debug, Clone)]
pub struct ContactCard {
    id: String,
    profile_img: String,
    name: String,
    date: String,
    last_message: String,
}
impl ContactCard {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        profile_image: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            profile_img: profile_image.into(),
            name: name.into(),
            date: "15/03/2023".into(),
            last_message: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateActiveId(String),
    ShowOnlyProfileImage,
    ShowFullCard,
}

#[derive(Debug, Clone)]
pub struct State {
    active_id: Option<String>,
    only_profile: bool,
    card: ContactCard,
}

impl State {
    pub fn new(card: ContactCard) -> Self {
        Self {
            active_id: None,
            only_profile: false,
            card,
        }
    }
    pub fn view(&self) -> Element<Message> {
        let mut is_active = false;
        if let Some(id) = &self.active_id {
            is_active = id == &self.card.id;
        }
        let btn_style = if is_active {
            iced::theme::Button::Custom(Box::new(ActiveButtonStyle {}))
        } else {
            iced::theme::Button::Custom(Box::new(ButtonStyle {}))
        };
        let btn_content: Element<_> = if self.only_profile {
            text(&self.card.profile_img).into()
        } else {
            row![
                text("Profile Image"),
                column![
                    text(&self.card.name),
                    text(&self.card.last_message)
                        .size(14.0)
                        .width(Length::Fill)
                        .height(Length::Fixed(30.0)),
                ],
                text(&self.card.date),
            ]
            .into()
        };
        button(btn_content)
            .width(Length::Fill)
            .height(Length::Fixed(80.0))
            .on_press(Message::UpdateActiveId(self.card.id.clone()))
            .style(btn_style)
            .into()
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::UpdateActiveId(id) => {
                self.active_id = Some(id);
            }
            Message::ShowOnlyProfileImage => {
                self.only_profile = true;
            }
            Message::ShowFullCard => {
                self.only_profile = false;
            }
        }
    }
}

struct ButtonStyle;
impl button::StyleSheet for ButtonStyle {
    type Style = iced::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: style.extended_palette().background.base.text,
            border_radius: 0.0,
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let plt = style.extended_palette();

        button::Appearance {
            background: Some(Color::from_rgb8(240, 240, 240).into()),
            text_color: plt.primary.weak.text,
            ..self.active(style)
        }
    }
}

struct ActiveButtonStyle;
impl button::StyleSheet for ActiveButtonStyle {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            border_radius: 0.0,
            background: Some(Color::from_rgb8(65, 159, 217).into()),
            text_color: Color::WHITE,
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            ..self.active(style)
        }
    }
}
