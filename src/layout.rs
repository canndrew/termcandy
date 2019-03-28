use futures::{Async, Future};
use crate::widget::Widget;
use crate::graphics::{Rect, SurfaceMut};

pub struct VecJoin<W> {
    widgets: Vec<(Rect, W)>,
}

impl<W> VecJoin<W> {
    pub fn new() -> VecJoin<W> {
        VecJoin {
            widgets: Vec::new(),
        }
    }

    pub fn push(&mut self, rect: Rect, widget: W) {
        self.widgets.push((rect, widget));
    }

    pub fn pop(&mut self) -> Option<(Rect, W)> {
        self.widgets.pop()
    }

    pub fn insert(&mut self, index: usize, rect: Rect, widget: W) {
        self.widgets.insert(index, (rect, widget));
    }

    pub fn swap_remove(&mut self, index: usize) -> (Rect, W) {
        self.widgets.swap_remove(index)
    }

    pub fn remove(&mut self, index: usize) -> (Rect, W) {
        self.widgets.remove(index)
    }

    pub fn widget(&self, index: usize) -> &W {
        let (_, widget) = &self.widgets[index];
        widget
    }

    pub fn widget_mut(&mut self, index: usize) -> &mut W {
        let (_, widget) = &mut self.widgets[index];
        widget
    }

    pub fn layout(&self, index: usize) -> &Rect {
        let (rect, _) = &self.widgets[index];
        rect
    }

    pub fn layout_mut(&mut self, index: usize) -> &mut Rect {
        let (rect, _) = &mut self.widgets[index];
        rect
    }
}

impl<W> Future for VecJoin<W>
where
    W: Widget,
    W::Item: Into<()>,
{
    type Item = ();
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<()>, W::Error> {
        let mut i = 0;
        while i < self.widgets.len() {
            let (rect, widget) = &mut self.widgets[i];
            match widget.map_mouse(|x, y| {
                if (x as i16) < rect.x0 || (x as i16) >= rect.x1 || (y as i16) < rect.y0 || (y as i16) >= rect.y1 {
                    None
                } else {
                    Some((x - rect.x0 as u16, y - rect.y0 as u16))
                }
            }).poll()? {
                Async::Ready(x) => x.into(),
                Async::NotReady => {
                    i += 1;
                    continue;
                },
            }
            let _ = self.widgets.remove(i);
        }
        if self.widgets.is_empty() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl<W> Widget for VecJoin<W>
where
    W: Widget,
    W::Item: Into<()>,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        for (rect, widget) in &self.widgets {
            let mut sub_surface = surface.region(*rect);
            widget.draw(&mut sub_surface);
        }
    }
}

