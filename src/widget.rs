use super::*;

use crate::graphics::{SurfaceMut, Rect};
use crate::input;
use termion::event::{MouseEvent, Event};

/// A `Widget` is a `Future` that can be drawn.
///
/// The easiest way to create widgets is using the `#[widget]` attribute.
pub trait Widget: Future {
    /// Draw the widget to the given surface.
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>);

    /// Filter and map all user input events arriving at the widget.
    fn map_events<M: Fn(Event) -> Option<Event>>(self, map: M) -> MapEvents<Self, M>
    where
        Self: Sized,
        M: Fn(Event) -> Option<Event> + Sync + Send,
    {
        MapEvents {
            widget: self,
            map: map,
        }
    }

    /// Resize the widget to the given region calculated from the width and height of the screen.
    ///
    /// The widget will draw itself to the region you calculate, the position of mouse-click
    /// events will be mapped accordingly, and `widget::screen_size()` will report the given region
    /// when called from inside the widget.
    fn resize<M: Fn(u16, u16) -> Rect>(self, map: M) -> Resize<Self, M>
    where
        Self: Sized,
        M: Fn(u16, u16) -> Rect + Sync + Send,
    {
        Resize {
            widget: self,
            map: map,
        }
    }
}

/// Extension trait for futures.
pub trait FutureExt: Future {
    /// Convert any future to a widget by giving it a draw method.
    fn draw_as<D>(self, drawer: D) -> DrawAs<Self, D>
    where
        Self: Sized,
        D: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
    {
        DrawAs {
            future: self,
            drawer,
        }
    }
}

#[pin_project]
pub struct DrawAs<F, D> {
    #[pin]
    future: F,
    drawer: D,
}

impl<F, D> Future for DrawAs<F, D>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.project();
        this.future.poll(cx)
    }
}

impl<F, D> Widget for DrawAs<F, D>
where
    F: Future,
    D: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        (self.drawer)(surface)
    }
}

impl<F> FutureExt for F
where
    F: Future
{}

impl<'a, W> Widget for &'a mut W
where
    W: Widget + Unpin,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        (**self).draw(surface)
    }
}

pub struct Draw<F> {
    draw: F,
}

/// Create a widget that never completes and draws itself using the given method.
//pub fn draw<F>(draw: F) -> impl Widget<Output = !>
pub fn draw<F>(draw: F) -> Draw<F>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    Draw {
        draw,
    }
}

impl<F> Future for Draw<F>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    type Output = !;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<!> {
        let ass: &Draw<F> = self.as_ref().get_ref();
        trace!("in Draw::poll, self == {:?}", ass as *const _);
        Poll::Pending
    }
}

impl<F> Widget for Draw<F>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        trace!("in Draw::draw, self == {:?}", self as *const _);
        (self.draw)(surface)
    }
}

/// Widget created using the `Widget::resize` method.
#[pin_project]
pub struct Resize<W, M> {
    #[pin]
    widget: W,
    map: M,
}

impl<W, M> Future for Resize<W, M>
where
    W: Future,
    M: Fn(u16, u16) -> Rect + Sync + Send,
{
    type Output = W::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<W::Output> {
        let this = self.project();
        let map = this.map;
        let (w, h) = crate::screen::screen_size();
        let area = map(w, h);
        let mapped_w = (area.x1 - area.x0) as u16;
        let mapped_h = (area.y1 - area.y0) as u16;
        let mouse_map = &|x, y| {
            if (x as i16) < area.x0 || (x as i16) >= area.x1 || (y as i16) < area.y0 || (y as i16) >= area.y1 {
                return None;
            }
            Some((x - area.x0 as u16, y - area.y0 as u16))
        };
        let widget = this.widget;
        crate::screen::with_screen_size(mapped_w, mapped_h, || {
            input::with_event_map(
                |event| Some(match event {
                    Event::Mouse(mouse_event) => Event::Mouse(match mouse_event {
                        MouseEvent::Press(button, x, y) => {
                            let (x, y) = mouse_map(x, y)?;
                            MouseEvent::Press(button, x, y)
                        },
                        MouseEvent::Release(x, y) => {
                            let (x, y) = mouse_map(x, y)?;
                            MouseEvent::Release(x, y)
                        },
                        MouseEvent::Hold(x, y) => {
                            let (x, y) = mouse_map(x, y)?;
                            MouseEvent::Hold(x, y)
                        },
                    }),
                    event => event,
                }),
                panic::AssertUnwindSafe(move || widget.poll(cx)),
            )
        })
    }
}

impl<W, M> Widget for Resize<W, M>
where
    W: Widget,
    M: Fn(u16, u16) -> Rect + Sync + Send,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        let rect = surface.rect();
        let mut sub_surface = surface.region((self.map)(rect.x1 as u16, rect.y1 as u16));
        self.widget.draw(&mut sub_surface)
    }
}

/// Widget created using the `Widget::map_events` method.
#[pin_project]
pub struct MapEvents<W, M> {
    #[pin]
    widget: W,
    map: M,
}

impl<W, M> Future for MapEvents<W, M>
where
    W: Future,
    M: Fn(Event) -> Option<Event> + Sync + Send,
{
    type Output = W::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<W::Output> {
        let this = self.project();
        let map = &*this.map;
        let widget = this.widget;
        input::with_event_map(map, panic::AssertUnwindSafe(|| widget.poll(cx)))
    }
}

impl<W, M> Widget for MapEvents<W, M>
where
    W: Widget,
    M: Fn(Event) -> Option<Event> + Sync + Send,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        self.widget.draw(surface)
    }
}

