use crate::graphics::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Style {
    pub fn bold() -> Style {
        Style {
            attrs: Attrs::bold(),
            .. Style::default()
        }
    }

    pub fn fg(color: Color) -> Style {
        Style {
            fg: color,
            .. Style::default()
        }
    }

    pub fn bg(color: Color) -> Style {
        Style {
            bg: color,
            .. Style::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Attrs {
    pub bold: bool,
}

impl Attrs {
    pub fn bold() -> Attrs {
        Attrs {
            bold: true,
            .. Attrs::default()
        }
    }
}

