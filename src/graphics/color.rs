/// A color
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Default terminal color. eg. use this to set transparent background.
    Default,
    /// 4-bit, 16-color mode colors.
    Colors16 {
        code: ColorCode,
        bright: bool,
    },
    /// 8-bit, 256-color mode colors.
    Colors256(u8),
    /// 24-bit, true-color mode colors.
    Rgb {
        r: u8,
        g: u8,
        b: u8,
    },
}

impl Default for Color {
    fn default() -> Color {
        Color::Default
    }
}

impl Color {
    pub fn black() -> Color {
        Color::Colors16 { code: ColorCode::Black, bright: false }
    }

    pub fn red() -> Color {
        Color::Colors16 { code: ColorCode::Red, bright: false }
    }

    pub fn green() -> Color {
        Color::Colors16 { code: ColorCode::Green, bright: false }
    }

    pub fn yellow() -> Color {
        Color::Colors16 { code: ColorCode::Yellow, bright: false }
    }

    pub fn blue() -> Color {
        Color::Colors16 { code: ColorCode::Blue, bright: false }
    }

    pub fn magenta() -> Color {
        Color::Colors16 { code: ColorCode::Magenta, bright: false }
    }

    pub fn cyan() -> Color {
        Color::Colors16 { code: ColorCode::Cyan, bright: false }
    }

    pub fn white() -> Color {
        Color::Colors16 { code: ColorCode::White, bright: false }
    }

    pub fn bright_black() -> Color {
        Color::Colors16 { code: ColorCode::Black, bright: true }
    }

    pub fn bright_red() -> Color {
        Color::Colors16 { code: ColorCode::Red, bright: true }
    }

    pub fn bright_green() -> Color {
        Color::Colors16 { code: ColorCode::Green, bright: true }
    }

    pub fn bright_yellow() -> Color {
        Color::Colors16 { code: ColorCode::Yellow, bright: true }
    }

    pub fn bright_blue() -> Color {
        Color::Colors16 { code: ColorCode::Blue, bright: true }
    }

    pub fn bright_magenta() -> Color {
        Color::Colors16 { code: ColorCode::Magenta, bright: true }
    }

    pub fn bright_cyan() -> Color {
        Color::Colors16 { code: ColorCode::Cyan, bright: true }
    }

    pub fn bright_white() -> Color {
        Color::Colors16 { code: ColorCode::White, bright: true }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::Rgb { r, b, g }
    }
}

/// A simple 3-bit color code.
///
/// These colors should be compatible with just about any terminal under the sun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorCode {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}

