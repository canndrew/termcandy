#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Named(u8, bool),
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
        Color::Named(0, false)
    }

    pub fn red() -> Color {
        Color::Named(1, false)
    }

    pub fn green() -> Color {
        Color::Named(2, false)
    }

    pub fn yellow() -> Color {
        Color::Named(3, false)
    }

    pub fn blue() -> Color {
        Color::Named(4, false)
    }

    pub fn magenta() -> Color {
        Color::Named(5, false)
    }

    pub fn cyan() -> Color {
        Color::Named(6, false)
    }

    pub fn white() -> Color {
        Color::Named(7, false)
    }

    pub fn bright_black() -> Color {
        Color::Named(0, true)
    }

    pub fn bright_red() -> Color {
        Color::Named(1, true)
    }

    pub fn bright_green() -> Color {
        Color::Named(2, true)
    }

    pub fn bright_yellow() -> Color {
        Color::Named(3, true)
    }

    pub fn bright_blue() -> Color {
        Color::Named(4, true)
    }

    pub fn bright_magenta() -> Color {
        Color::Named(5, true)
    }

    pub fn bright_cyan() -> Color {
        Color::Named(6, true)
    }

    pub fn bright_white() -> Color {
        Color::Named(7, true)
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::Rgb { r, b, g }
    }
}

