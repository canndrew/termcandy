use crate::graphics::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Attrs {
    bold: bool,
}

