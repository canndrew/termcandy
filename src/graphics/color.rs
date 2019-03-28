#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Colors16(u8, bool),
    Colors256(u8),
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
        Color::Colors16(0, false)
    }

    pub fn red() -> Color {
        Color::Colors16(1, false)
    }

    pub fn green() -> Color {
        Color::Colors16(2, false)
    }

    pub fn yellow() -> Color {
        Color::Colors16(3, false)
    }

    pub fn blue() -> Color {
        Color::Colors16(4, false)
    }

    pub fn magenta() -> Color {
        Color::Colors16(5, false)
    }

    pub fn cyan() -> Color {
        Color::Colors16(6, false)
    }

    pub fn white() -> Color {
        Color::Colors16(7, false)
    }

    pub fn bright_black() -> Color {
        Color::Colors16(0, true)
    }

    pub fn bright_red() -> Color {
        Color::Colors16(1, true)
    }

    pub fn bright_green() -> Color {
        Color::Colors16(2, true)
    }

    pub fn bright_yellow() -> Color {
        Color::Colors16(3, true)
    }

    pub fn bright_blue() -> Color {
        Color::Colors16(4, true)
    }

    pub fn bright_magenta() -> Color {
        Color::Colors16(5, true)
    }

    pub fn bright_cyan() -> Color {
        Color::Colors16(6, true)
    }

    pub fn bright_white() -> Color {
        Color::Colors16(7, true)
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::Rgb { r, b, g }
    }
}

