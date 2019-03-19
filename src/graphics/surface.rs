use crate::graphics::{Style, Rect};
use unicode_width::UnicodeWidthChar;
use std::cmp;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Cell {
    pub style: Style,
    pub c: char,
}

pub struct Surface {
    w: u16,
    h: u16,
    cells: Vec<Cell>,
}

impl Surface {
    pub fn blank(w: u16, h: u16) -> Surface {
        let mut cells = Vec::with_capacity(w as usize * h as usize);
        for _ in 0..(w as usize * h as usize) {
            cells.push(Cell { style: Style::default(), c: ' ' });
        }
        Surface {
            w: w,
            h: h,
            cells: cells,
        }
    }

    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        let i = self.index(x as i16, y as i16).unwrap();
        &self.cells[i]
    }

    pub fn cell_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let i = self.index(x as i16, y as i16).unwrap();
        &mut self.cells[i]
    }

    pub fn put(&mut self, c: char, x: i16, y: i16, style: Style) {
        let i = match self.index(x, y) {
            Some(i) => i,
            None => return,
        };
        self.cells[i].c = c;
        self.cells[i].style = style;
        for n in 1..(c.width().unwrap_or(0) as i16) {
            let i = match self.index(x + n, y) {
                Some(i) => i,
                None => return,
            };
            self.cells[i].c = '\0';
            self.cells[i].style = style;
        }
    }

    pub fn print(&mut self, text: &str, x: i16, y: i16, style: Style) {
        let w = self.w as i16;
        self.print_inner(text, x, w, y, style)
    }

    fn print_inner(&mut self, text: &str, x0: i16, x1: i16, y: i16, style: Style) {
        let mut chars = text.chars();
        let mut x = x0;
        while let Some(c) = chars.next() {
            let width = c.width().unwrap_or(0) as i16;
            if x + width >= x1 {
                break;
            }
            self.put(c, x, y, style);
            x += width;
        }
    }

    pub fn width(&self) -> u16 {
        self.w
    }

    pub fn height(&self) -> u16 {
        self.h
    }

    pub fn as_ref(&self) -> SurfaceRef {
        SurfaceRef {
            surface: self,
            rect: Rect {
                x0: 0,
                y0: 0,
                x1: self.w as i16,
                y1: self.h as i16,
            },
        }
    }

    pub fn as_mut(&mut self) -> SurfaceMut {
        let w = self.w as i16;
        let h = self.h as i16;
        SurfaceMut {
            surface: self,
            rect: Rect {
                x0: 0,
                y0: 0,
                x1: w,
                y1: h,
            },
        }
    }

    pub fn draw_h_line(&mut self, x0: i16, x1: i16, y: i16) {
        for x in x0..x1 {
            let i = match self.index(x, y) {
                Some(i) => i,
                None => break,
            };
            let c = self.cells[i].c;
            let v = char_to_segments(c).unwrap_or(0);
            let v = v | 0b0001;
            let c = segments_to_char(v);
            self.cells[i].c = c;
        }
        for x in (x0 + 1)..(x1 + 1) {
            let i = match self.index(x, y) {
                Some(i) => i,
                None => break,
            };
            let c = self.cells[i].c;
            let v = char_to_segments(c).unwrap_or(0);
            let v = v | 0b0100;
            let c = segments_to_char(v);
            self.cells[i].c = c;
        }
    }

    pub fn draw_v_line(&mut self, y0: i16, y1: i16, x: i16) {
        for y in y0..y1 {
            let i = match self.index(x, y) {
                Some(i) => i,
                None => break,
            };
            let c = self.cells[i].c;
            let v = char_to_segments(c).unwrap_or(0);
            let v = v | 0b1000;
            let c = segments_to_char(v);
            self.cells[i].c = c;
        }
        for y in (y0 + 1)..(y1 + 1) {
            let i = match self.index(x, y) {
                Some(i) => i,
                None => break,
            };
            let c = self.cells[i].c;
            let v = char_to_segments(c).unwrap_or(0);
            let v = v | 0b0010;
            let c = segments_to_char(v);
            self.cells[i].c = c;
        }
    }

    fn index(&self, x: i16, y: i16) -> Option<usize> {
        let i = x as isize + y as isize * self.w as isize;
        if i >= 0 && ((i as usize) < self.cells.len()) {
            Some(i as usize)
        } else {
            None
        }
    }
}

pub struct SurfaceRef<'a> {
    surface: &'a Surface,
    rect: Rect,
}

impl<'a> SurfaceRef<'a> {
    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        self.surface.cell((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    pub fn width(&self) -> u16 {
        self.rect.width()
    }

    pub fn height(&self) -> u16 {
        self.rect.height()
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x0: 0,
            x1: self.width() as i16,
            y0: 0,
            y1: self.height() as i16,
        }
    }
}

pub struct SurfaceMut<'a> {
    surface: &'a mut Surface,
    rect: Rect,
}

impl<'a> SurfaceMut<'a> {
    pub fn print(&mut self, text: &str, x: i16, y: i16, style: Style) {
        self.surface.print_inner(text, x + self.rect.x0, self.rect.x1, y + self.rect.y0, style)
    }

    pub fn width(&self) -> u16 {
        self.rect.width()
    }

    pub fn height(&self) -> u16 {
        self.rect.height()
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x0: 0,
            x1: self.width() as i16,
            y0: 0,
            y1: self.height() as i16,
        }
    }

    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        self.surface.cell((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    pub fn cell_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        self.surface.cell_mut((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    pub fn draw_h_line(&mut self, x0: i16, x1: i16, y: i16) {
        let x0 = cmp::max(-1, x0);
        let x1 = cmp::min(self.width() as i16, x1);
        let y = cmp::max(-1, y);
        let y = cmp::min(self.height() as i16, y);
        self.surface.draw_h_line(x0 + self.rect.x0, x1 + self.rect.x0, y + self.rect.y0);
    }

    pub fn draw_v_line(&mut self, y0: i16, y1: i16, x: i16) {
        let y0 = cmp::max(-1, y0);
        let y1 = cmp::min(self.height() as i16, y1);
        let x = cmp::max(-1, x);
        let x = cmp::min(self.width() as i16, x);
        self.surface.draw_v_line(y0 + self.rect.y0, y1 + self.rect.y0, x + self.rect.x0);
    }

    pub fn region(&mut self, rect: Rect) -> SurfaceMut {
        assert!(rect.x0 >= 0);
        assert!(rect.x1 >= rect.x0 && rect.x1 <= self.width() as i16);
        assert!(rect.y0 >= 0);
        assert!(rect.y1 >= rect.y0 && rect.y1 <= self.height() as i16);
        SurfaceMut {
            surface: self.surface,
            rect: Rect {
                x0: self.rect.x0 + rect.x0,
                x1: self.rect.x0 + rect.x1,
                y0: self.rect.y0 + rect.y0,
                y1: self.rect.y0 + rect.y1,
            },
        }
    }
}

fn char_to_segments(c: char) -> Option<u8> {
    let v = match c {
        '╶' => 0b0001,
        '╵' => 0b0010,
        '╴' => 0b0100,
        '╷' => 0b1000,
        '└' => 0b0011,
        '─' => 0b0101,
        '┌' => 0b1001,
        '┘' => 0b0110,
        '│' => 0b1010,
        '┐' => 0b1100,
        '┴' => 0b0111,
        '├' => 0b1011,
        '┬' => 0b1101,
        '┤' => 0b1110,
        '┼' => 0b1111,
        _   => return None,
    };
    Some(v)
}

fn segments_to_char(v: u8) -> char {
    match v {
        0b0001 => '╶',
        0b0010 => '╵',
        0b0100 => '╴',
        0b1000 => '╷',
        0b0011 => '└',
        0b0101 => '─',
        0b1001 => '┌',
        0b0110 => '┘',
        0b1010 => '│',
        0b1100 => '┐',
        0b0111 => '┴',
        0b1011 => '├',
        0b1101 => '┬',
        0b1110 => '┤',
        0b1111 => '┼',
        _   => panic!(),
    }
}

