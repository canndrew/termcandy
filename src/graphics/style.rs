use super::*;

/// The color and styling attributes of a terminal cell.
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

    pub fn underline(style: UnderlineStyle) -> Style {
        Style {
            attrs: Attrs::underline(style),
            .. Style::default()
        }
    }

    pub fn faint() -> Style {
        Style {
            attrs: Attrs::faint(),
            .. Style::default()
        }
    }

    pub fn italic() -> Style {
        Style {
            attrs: Attrs::italic(),
            .. Style::default()
        }
    }

    pub fn blink() -> Style {
        Style {
            attrs: Attrs::blink(),
            .. Style::default()
        }
    }

    pub fn strikethrough() -> Style {
        Style {
            attrs: Attrs::strikethrough(),
            .. Style::default()
        }
    }

    pub fn overlined() -> Style {
        Style {
            attrs: Attrs::overlined(),
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

/// Styling attributes - bold, underlined, etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Attrs {
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: Option<UnderlineStyle>,
    pub blink: bool,
    pub strikethrough: bool,
    pub overlined: bool,
}

impl Attrs {
    pub fn underline(style: UnderlineStyle) -> Attrs {
        Attrs {
            underline: Some(style),
            .. Attrs::default()
        }
    }

    pub fn bold() -> Attrs {
        Attrs {
            bold: true,
            .. Attrs::default()
        }
    }

    pub fn faint() -> Attrs {
        Attrs {
            faint: true,
            .. Attrs::default()
        }
    }

    pub fn italic() -> Attrs {
        Attrs {
            italic: true,
            .. Attrs::default()
        }
    }

    pub fn blink() -> Attrs {
        Attrs {
            blink: true,
            .. Attrs::default()
        }
    }

    pub fn strikethrough() -> Attrs {
        Attrs {
            strikethrough: true,
            .. Attrs::default()
        }
    }

    pub fn overlined() -> Attrs {
        Attrs {
            overlined: true,
            .. Attrs::default()
        }
    }
}

/// The style of an underline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnderlineStyle {
    pub kind: UnderlineKind,
    pub color: Color,
}

impl UnderlineStyle {
    pub fn single() -> UnderlineStyle {
        UnderlineStyle {
            kind: UnderlineKind::Single,
            color: Color::default(),
        }
    }

    pub fn double() -> UnderlineStyle {
        UnderlineStyle {
            kind: UnderlineKind::Double,
            color: Color::default(),
        }
    }

    pub fn wavy() -> UnderlineStyle {
        UnderlineStyle {
            kind: UnderlineKind::Wavy,
            color: Color::default(),
        }
    }
}

/// The kind of underline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnderlineKind {
    Single,
    Double,
    Wavy,
}

