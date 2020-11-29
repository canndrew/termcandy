use super::*;

/// A single grid cell of text on the terminal.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Cell {
    pub style: Style,
    pub c: char,
}

/// A 2-dimensional arrays of cells.
pub struct Surface {
    w: u16,
    h: u16,
    cells: Vec<Cell>,
}

impl Surface {
    /// Create a new, blank surface.
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

    /// Get a reference to the cell at position (x, y).
    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        let i = self.index(x as i16, y as i16).unwrap();
        &self.cells[i]
    }

    /// Get a mutable reference to the cell at position (x, y).
    pub fn cell_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let i = self.index(x as i16, y as i16).unwrap();
        &mut self.cells[i]
    }

    /// Set the cell at position (x, y)
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

    /// Print text to the surface.
    pub fn print(&mut self, text: &str, x: i16, y: i16, style: Style) {
        let w = self.w as i16;
        self.print_inner(text, x, w, y, style)
    }

    fn print_inner(&mut self, text: &str, x0: i16, x1: i16, y: i16, style: Style) {
        let mut chars = text.chars();
        let mut x = x0;
        while let Some(c) = chars.next() {
            let width = c.width().unwrap_or(0) as i16;
            if x + width > x1 {
                break;
            }
            self.put(c, x, y, style);
            x += width;
        }
    }

    /// Get the surface's width
    pub fn width(&self) -> u16 {
        self.w
    }

    /// Get the surface's height
    pub fn height(&self) -> u16 {
        self.h
    }

    /// Get a `SurfaceRef` reference to the surface.
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

    /// Get a `SurfaceMut` reference to the surface.
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

    /// Draw a horizonal line on the surface.
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

    /// Draw a vertical line on the surface.
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
        if x < 0 || x as u16 >= self.w || y < 0 || y as u16 >= self.h {
            return None;
        }

        let i = x as isize + y as isize * self.w as isize;
        Some(i as usize)
    }

    fn clear(&mut self, rect: Rect) {
        let y0 = cmp::max(0, rect.y0) as usize;
        let y1 = cmp::min(self.h as i16, rect.y1) as usize;
        let x0 = cmp::max(0, rect.x0) as usize;
        let x1 = cmp::min(self.w as i16, rect.x1) as usize;

        let w = self.w as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let cell = &mut self.cells[x + y * w];
                cell.style = Style::default();
                cell.c = ' ';
            }
        }
    }
}

/// A shared reference to a (region of a) surface.
pub struct SurfaceRef<'a> {
    surface: &'a Surface,
    rect: Rect,
}

impl<'a> SurfaceRef<'a> {
    /// Get a reference to the cell at position (x, y).
    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        self.surface.cell((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    /// Get the surface's width
    pub fn width(&self) -> u16 {
        self.rect.width()
    }

    /// Get the surface's height
    pub fn height(&self) -> u16 {
        self.rect.height()
    }

    /// Get a rectangle representing the entire area of the surface.
    pub fn rect(&self) -> Rect {
        Rect {
            x0: 0,
            x1: self.width() as i16,
            y0: 0,
            y1: self.height() as i16,
        }
    }
}

/// A mutable reference to a (region of a) surface.
pub struct SurfaceMut<'a> {
    surface: &'a mut Surface,
    rect: Rect,
}

impl<'a> SurfaceMut<'a> {
    pub fn print(&mut self, text: &str, x: i16, y: i16, style: Style) {
        self.surface.print_inner(text, x + self.rect.x0, self.rect.x1, y + self.rect.y0, style)
    }

    /// Get the surface's width
    pub fn width(&self) -> u16 {
        self.rect.width()
    }

    /// Get the surface's height
    pub fn height(&self) -> u16 {
        self.rect.height()
    }

    /// Get a rectangle representing the entire area of the surface.
    pub fn rect(&self) -> Rect {
        Rect {
            x0: 0,
            x1: self.width() as i16,
            y0: 0,
            y1: self.height() as i16,
        }
    }

    /// Get a reference to the cell at position (x, y).
    pub fn cell(&self, x: u16, y: u16) -> &Cell {
        self.surface.cell((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    /// Get a mutable reference to the cell at position (x, y).
    pub fn cell_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        self.surface.cell_mut((x as i16 + self.rect.x0) as u16, (y as i16 + self.rect.y0) as u16)
    }

    /// Draw a horizontal line on the surface.
    pub fn draw_h_line(&mut self, x0: i16, x1: i16, y: i16) {
        let x0 = cmp::max(-1, x0);
        let x1 = cmp::min(self.width() as i16, x1);
        let y = cmp::max(-1, y);
        let y = cmp::min(self.height() as i16, y);
        self.surface.draw_h_line(x0 + self.rect.x0, x1 + self.rect.x0, y + self.rect.y0);
    }

    /// Draw a vertical line on the surface.
    pub fn draw_v_line(&mut self, y0: i16, y1: i16, x: i16) {
        let y0 = cmp::max(-1, y0);
        let y1 = cmp::min(self.height() as i16, y1);
        let x = cmp::max(-1, x);
        let x = cmp::min(self.width() as i16, x);
        self.surface.draw_v_line(y0 + self.rect.y0, y1 + self.rect.y0, x + self.rect.x0);
    }

    /// Get a sub-region of the surface.
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

    /// Clear the surface.
    pub fn clear(&mut self) {
        self.surface.clear(self.rect);
    }

    /// Fill the surface with the given color.
    pub fn fill(&mut self, color: Color) {
        for y in self.rect.y0..self.rect.y1 {
            for x in self.rect.x0..self.rect.x1 {
                self.surface.put(' ', x, y, Style::bg(color));
            }
        }
    }

    /// Print a single character to a position on the surface.
    pub fn put(&mut self, c: char, x: i16, y: i16, style: Style) {
        if x < 0 || x >= self.width() as i16 || y < 0 || y >= self.height() as i16 {
            return;
        }
        self.surface.put(c, x + self.rect.x0, y + self.rect.y0, style);
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

