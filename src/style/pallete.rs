use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppPalette {
    pub background: Color,
    pub chat_bg: Color,
    pub primary: Color,
    pub primary_lighter: Color,
    pub danger: Color,
    pub text_color: Color,
    pub grayish: Color,
    pub primary_darker: Color,
    pub hovered_bg_scrollbar: Color,
    pub hovered_bg_scroller: Color,
    pub hovered_bg_scrollbar_mo: Color,
    pub hovered_bg_scroller_mo: Color,
    pub primary_opaque: Color,
    pub chat_search_input_bg: Color,
    pub status_bar_bg: Color,
    pub status_bar_text_color: Color,
    pub hover_status_bar_bg: Color,
    pub welcome_bg_1: Color,
    pub welcome_bg_2: Color,
    pub welcome_bg_3: Color,
    pub notification: Color,
}
impl AppPalette {
    pub const LIGHT: Self = Self {
        background: Color::from_rgb(227.0 / 255.0, 229.0 / 255.0, 231.0 / 255.0),
        chat_bg: Color::from_rgb(253.0 / 255.0, 255.0 / 255.0, 248.0 / 255.0),
        primary: Color::from_rgb(82.0 / 255.0, 136.0 / 255.0, 193.0 / 255.0),
        primary_lighter: Color::from_rgb(105.0 / 255.0, 174.0 / 255.0, 247.0 / 255.0),
        primary_darker: Color::from_rgb(43.0 / 255.0, 82.0 / 255.0, 120.0 / 255.0),
        primary_opaque: Color::from_rgb(125.0 / 255.0, 168.0 / 255.0, 211.0 / 255.0),
        danger: Color::from_rgb(246.0 / 255.0, 50.0 / 255.0, 126.0 / 255.0),
        text_color: Color::from_rgb(11.0 / 255.0, 11.0 / 255.0, 11.0 / 255.0),
        grayish: Color::from_rgb(108.0 / 255.0, 120.0 / 255.0, 131.0 / 255.0),

        chat_search_input_bg: Color::from_rgb(246.0 / 255.0, 245.0 / 255.0, 239.0 / 255.0),
        hover_status_bar_bg: Color::from_rgb(52.0 / 255.0, 53.0 / 255.0, 59.0 / 255.0),
        status_bar_bg: Color::from_rgb(25.0 / 255.0, 26.0 / 255.0, 33.0 / 255.0),
        status_bar_text_color: Color::from_rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),

        welcome_bg_1: Color::from_rgb(42.0 / 255.0, 37.0 / 255.0, 120.0 / 255.0),
        welcome_bg_2: Color::from_rgb(26.0 / 255.0, 36.0 / 255.0, 56.0 / 255.0),
        welcome_bg_3: Color::from_rgb(16.0 / 255.0, 21.0 / 255.0, 60.0 / 255.0),
        notification: Color::from_rgb(64.0 / 255.0, 130.0 / 255.0, 188.0 / 255.0),

        hovered_bg_scrollbar: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.3),
        hovered_bg_scroller: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.3),
        hovered_bg_scrollbar_mo: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.6),
        hovered_bg_scroller_mo: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.6),
        // notification: Color::from_rgb(9.0 / 255.0, 211.0 / 255.0, 245.0 / 255.0),
    };
    pub const DARK: Self = Self {
        background: Color::from_rgb(23.0 / 255.0, 33.0 / 255.0, 43.0 / 255.0),
        chat_bg: Color::from_rgb(14.0 / 255.0, 22.0 / 255.0, 33.0 / 255.0),
        primary: Color::from_rgb(82.0 / 255.0, 136.0 / 255.0, 193.0 / 255.0),
        primary_lighter: Color::from_rgb(105.0 / 255.0, 174.0 / 255.0, 247.0 / 255.0),
        primary_darker: Color::from_rgb(43.0 / 255.0, 82.0 / 255.0, 120.0 / 255.0),
        primary_opaque: Color::from_rgb(125.0 / 255.0, 168.0 / 255.0, 211.0 / 255.0),

        danger: Color::from_rgb(246.0 / 255.0, 50.0 / 255.0, 126.0 / 255.0),
        text_color: Color::from_rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),
        grayish: Color::from_rgb(108.0 / 255.0, 120.0 / 255.0, 131.0 / 255.0),

        chat_search_input_bg: Color::from_rgb(36.0 / 255.0, 47.0 / 255.0, 61.0 / 255.0),
        hover_status_bar_bg: Color::from_rgb(52.0 / 255.0, 53.0 / 255.0, 59.0 / 255.0),
        status_bar_bg: Color::from_rgb(25.0 / 255.0, 26.0 / 255.0, 33.0 / 255.0),
        status_bar_text_color: Color::from_rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),

        welcome_bg_1: Color::from_rgb(42.0 / 255.0, 37.0 / 255.0, 120.0 / 255.0),
        welcome_bg_2: Color::from_rgb(26.0 / 255.0, 36.0 / 255.0, 56.0 / 255.0),
        welcome_bg_3: Color::from_rgb(16.0 / 255.0, 21.0 / 255.0, 60.0 / 255.0),
        notification: Color::from_rgb(64.0 / 255.0, 130.0 / 255.0, 188.0 / 255.0),

        hovered_bg_scrollbar: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.1),
        hovered_bg_scroller: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.2),
        hovered_bg_scrollbar_mo: Color::from_rgba(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0, 0.2),
        hovered_bg_scroller_mo: Color::from_rgba(255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.5),
    };
}
