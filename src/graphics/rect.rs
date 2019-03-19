use std::cmp;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x0: i16,
    pub x1: i16,
    pub y0: i16,
    pub y1: i16,
}

impl Rect {
    pub fn width(&self) -> u16 {
        (self.x1 - self.x0) as u16
    }

    pub fn height(&self) -> u16 {
        (self.y1 - self.y0) as u16
    }

    pub fn shrink_left(&mut self, amount: i16) {
        self.x0 += amount;
        self.x0 = cmp::min(self.x0, self.x1);
    }

    pub fn shrink_right(&mut self, amount: i16) {
        self.x1 -= amount;
        self.x1 = cmp::max(self.x0, self.x1);
    }

    pub fn shrink_top(&mut self, amount: i16) {
        self.y0 += amount;
        self.y0 = cmp::min(self.y0, self.y1);
    }

    pub fn shrink_bottom(&mut self, amount: i16) {
        self.y1 -= amount;
        self.y1 = cmp::max(self.y0, self.y1);
    }
}

