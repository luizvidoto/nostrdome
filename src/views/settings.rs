use iced::widget::text;
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub enum State {
    ABC,
}
impl Default for State {
    fn default() -> Self {
        Self::ABC
    }
}
impl State {
    pub fn view(&self) -> Element<Message> {
        text("Settings").into()
    }
    pub fn update(&mut self, message: Message) {
        match message {}
    }
}
