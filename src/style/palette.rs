use crate::error::Error;
use iced::{color, Color};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeMeta {
    pub theme_type: ThemeType,
}

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub enum Theme {
    #[default]
    Dark,
    Dracula,
    Catppuccin,
    Nord,
    LMMS,
    OneShot,
    LightWhats,
    LightGram,
    LightCord,
}

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8((*self).into())
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let i = u8::deserialize(deserializer)?;
        Ok(i.into())
    }
}

impl Theme {
    pub const ALL: [Self; 9] = [
        Self::Dark,
        Self::Dracula,
        Self::Catppuccin,
        Self::Nord,
        Self::LMMS,
        Self::OneShot,
        Self::LightWhats,
        Self::LightGram,
        Self::LightCord,
    ];

    pub const DARK: [Self; 6] = [
        Self::Dark,
        Self::Dracula,
        Self::Catppuccin,
        Self::Nord,
        Self::LMMS,
        Self::OneShot,
    ];
    pub const LIGHT: [Self; 3] = [Self::LightWhats, Self::LightGram, Self::LightCord];

    pub fn theme_meta(&self) -> ThemeMeta {
        match self {
            Theme::Dark => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::Dracula => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::Catppuccin => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::Nord => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::LMMS => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::OneShot => ThemeMeta {
                theme_type: ThemeType::Dark,
            },
            Theme::LightWhats => ThemeMeta {
                theme_type: ThemeType::Light,
            },
            Theme::LightGram => ThemeMeta {
                theme_type: ThemeType::Light,
            },
            Theme::LightCord => ThemeMeta {
                theme_type: ThemeType::Light,
            },
        }
    }

    pub fn palette(&self) -> ColorPalette {
        match self {
            Self::Dark => ColorPalette {
                base: BaseColors {
                    background: color!(0x272727),
                    foreground: color!(0x353535),
                    text: color!(0xE0E0E0),
                    comment: color!(0x737373),
                },
                normal: NormalColors {
                    primary: color!(0x6f3380),
                    primary_variant: color!(0x4a2854),
                    secondary: color!(0x386e50),
                    error: color!(0xff5555),
                    success: color!(0x50fa7b),
                },
            },
            Self::Dracula => ColorPalette {
                base: BaseColors {
                    background: color!(0x282a36),
                    foreground: color!(0x44475a),
                    text: color!(0xf8f8f2),
                    comment: color!(0x6272a4),
                },
                normal: NormalColors {
                    primary: color!(0xff79c6),
                    primary_variant: color!(0xa65683),
                    secondary: color!(0x50fa7b),
                    error: color!(0xff5555),
                    success: color!(0x50fa7b),
                },
            },
            Self::LMMS => ColorPalette {
                base: BaseColors {
                    background: color!(0x26_2B_30),
                    foreground: color!(0x3B424A), //3B424A
                    text: color!(0xe5e9f0),
                    comment: color!(0x4a5f82),
                },
                normal: NormalColors {
                    primary: color!(0x309655),
                    primary_variant: color!(0x215233),
                    secondary: color!(0x309655),
                    error: color!(0xff5555),
                    success: color!(0x50fa7b),
                },
            },
            Self::Nord => ColorPalette {
                base: BaseColors {
                    background: color!(0x2e3440),
                    foreground: color!(0x3b4252),
                    text: color!(0xe5e9f0),
                    comment: color!(0x4a5f82),
                },
                normal: NormalColors {
                    primary: color!(0x88c0d0),
                    primary_variant: color!(0x6c8d96),
                    secondary: color!(0xa3be8c),
                    error: color!(0xbf616a),
                    success: color!(0x50fa7b),
                },
            },
            Self::OneShot => ColorPalette {
                base: BaseColors {
                    background: color!(0x1A0B1D),
                    foreground: color!(0x2B0D1A),
                    text: color!(0xFEFECD),
                    comment: color!(0x7d7d03),
                },
                normal: NormalColors {
                    primary: color!(0xF48550),
                    primary_variant: color!(0xa66446),
                    secondary: color!(0x80FF80),
                    error: color!(0xff5555),
                    success: color!(0x50fa7b),
                },
            },
            Self::Catppuccin => ColorPalette {
                base: BaseColors {
                    background: color!(0x1E1E28),
                    foreground: color!(0x332E41),
                    text: color!(0xFEFECD),
                    comment: color!(0x7d7d03),
                },
                normal: NormalColors {
                    primary: color!(0xC6AAE8),
                    primary_variant: color!(0x827394),
                    secondary: color!(0xB1E3AD),
                    error: color!(0xE38C8F),
                    success: color!(0x50fa7b),
                },
            },
            Self::LightWhats => ColorPalette {
                base: BaseColors {
                    background: color!(0xF3F3F3),
                    foreground: color!(0xFCFCFC),
                    text: color!(0x000000),
                    comment: color!(0x737373),
                },
                normal: NormalColors {
                    primary: color!(0x25D366),
                    primary_variant: color!(0xb7ebca),
                    secondary: color!(0x6625D4),
                    error: color!(0xD4255A), //FE3B30
                    success: color!(0x25D366),
                },
            },
            Self::LightGram => ColorPalette {
                base: BaseColors {
                    background: color!(0xF1F1F1),
                    foreground: color!(0xFFFFFF),
                    text: color!(0x000000),
                    comment: color!(0xAB9999),
                },
                normal: NormalColors {
                    primary: color!(0x40A7E3),
                    primary_variant: color!(0x9DD9FC),
                    secondary: color!(0xB580E2),
                    error: color!(0xD14E4E),
                    success: color!(0x25D366),
                },
            },
            Self::LightCord => ColorPalette {
                base: BaseColors {
                    background: color!(0xE3E5E8),
                    foreground: color!(0xFFFFFF),
                    text: color!(0x060607),
                    comment: color!(0x6D6F78),
                },
                normal: NormalColors {
                    primary: color!(0x5865F2),
                    primary_variant: color!(0xB6BCFC),
                    secondary: color!(0x23A55A),
                    error: color!(0xDA373C),
                    success: color!(0x23A55A),
                },
            },
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Theme::Dark => "Dark",
                Theme::Dracula => "Dracula",
                Theme::Nord => "Nord",
                Theme::LMMS => "LMMS",
                Theme::OneShot => "OneShot",
                Theme::Catppuccin => "Catppuccin",
                Theme::LightWhats => "Light Whats",
                Theme::LightGram => "Light Gram",
                Theme::LightCord => "Light Cord",
            }
        )
    }
}

impl From<Theme> for u8 {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => 0,
            Theme::Dracula => 1,
            Theme::Catppuccin => 2,
            Theme::Nord => 3,
            Theme::LMMS => 4,
            Theme::OneShot => 5,
            Theme::LightWhats => 6,
            Theme::LightGram => 7,
            Theme::LightCord => 8,
        }
    }
}
impl From<u8> for Theme {
    fn from(item: u8) -> Self {
        match item {
            0 => Theme::Dark,
            1 => Theme::Dracula,
            2 => Theme::Catppuccin,
            3 => Theme::Nord,
            4 => Theme::LMMS,
            5 => Theme::OneShot,
            6 => Theme::LightWhats,
            7 => Theme::LightGram,
            8 => Theme::LightCord,
            _ => Theme::default(), // returns the default theme (Dark in this case) if an unknown value is provided
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BaseColors {
    pub background: Color,
    pub foreground: Color,
    pub text: Color,
    pub comment: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct NormalColors {
    pub primary: Color,
    pub primary_variant: Color,
    pub secondary: Color,
    pub error: Color,
    pub success: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorPalette {
    pub base: BaseColors,
    pub normal: NormalColors,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Theme::default().palette()
    }
}

/*
    primary: hsb(167, 100, 66)
    primary_variant: hsb(169, 97, 36)
*/
