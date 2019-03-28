use std::ops::{Generator, GeneratorState};
use std::marker::PhantomPinned;
use futures::{Async, Future};
use crate::graphics::{SurfaceMut, Rect};
pub use termcandy_macros::widget;
use crate::events;
use std::pin::Pin;
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

#[doc(hidden)]
pub struct GenWidget<G> {
    gen: G,
    pub drawer: Option<Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>>,
    _pinned: PhantomPinned,
}

impl<G> GenWidget<G> {
    pub fn new(gen: G) -> GenWidget<G> {
        GenWidget {
            drawer: None,
            gen: gen,
            _pinned: std::marker::PhantomPinned,
        }
    }
}

#[doc(hidden)]
pub fn nil_drawer<'s, 'm>(_: &'m mut SurfaceMut<'s>) {}

impl<T, E, G> Future for GenWidget<G>
where
    G: Generator<Yield = Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>, Return = Result<T, E>>
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Result<Async<T>, E> {
        drop(self.drawer.take());
        let gen = unsafe { Pin::new_unchecked(&mut self.gen) };
        match gen.resume() {
            GeneratorState::Yielded(drawer) => {
                self.drawer = Some(drawer);
                Ok(Async::NotReady)
            },
            GeneratorState::Complete(val) => Ok(Async::Ready(val?)),
        }
    }
}

impl<T, E, G> Widget for GenWidget<G>
where
    G: Generator<Yield = Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>, Return = Result<T, E>>
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        if let Some(drawer) = &self.drawer {
            drawer(surface)
        }
    }
}

impl<G> Drop for GenWidget<G> {
    fn drop(&mut self) {
        // need to make sure this gets dropped first.
        // since it can reference gen
        drop(self.drawer.take());
    }
}

#[doc(hidden)]
pub unsafe fn forge_lifetime<'a>(b: Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'a>)
    -> Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>
{
    std::mem::transmute(b)
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

pub struct DrawAs<F, D> {
    future: F,
    drawer: D,
}

impl<F, D> Future for DrawAs<F, D>
where
    F: Future,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Result<Async<F::Item>, F::Error> {
        self.future.poll()
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
    W: Widget,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        (**self).draw(surface)
    }
}

#[doc(hidden)]
pub trait SelectDraw: Future {
    fn select_draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>);
}

impl<W: Future> SelectDraw for W {
    default fn select_draw<'s, 'm>(&self, _surface: &'m mut SurfaceMut<'s>) {
    }
}

impl<W: Widget> SelectDraw for W {
    fn select_draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        self.draw(surface)
    }
}

struct DrawWidget<F> {
    draw: F,
}

/// Create a widget that never completes and draws itself using the given method.
pub fn draw<F>(draw: F) -> impl Widget<Item = !, Error = !>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    DrawWidget {
        draw,
    }
}

impl<F> Future for DrawWidget<F>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    type Item = !;
    type Error = !;

    fn poll(&mut self) -> Result<Async<!>, !> {
        Ok(Async::NotReady)
    }
}

impl<F> Widget for DrawWidget<F>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>),
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        (self.draw)(surface)
    }
}

/// Widget created using the `Widget::resize` method.
pub struct Resize<W, M> {
    widget: W,
    map: M,
}

impl<W, M> Future for Resize<W, M>
where
    W: Future,
    M: Fn(u16, u16) -> Rect + Sync + Send,
{
    type Item = W::Item;
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<W::Item>, W::Error> {
        let map = &self.map;
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
        let widget = &mut self.widget;
        crate::screen::with_screen_size(mapped_w, mapped_h, || {
            events::with_event_map(
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
                || widget.poll(),
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
pub struct MapEvents<W, M> {
    widget: W,
    map: M,
}

impl<W, M> Future for MapEvents<W, M>
where
    W: Future,
    M: Fn(Event) -> Option<Event> + Sync + Send,
{
    type Item = W::Item;
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<W::Item>, W::Error> {
        let map = &self.map;
        let widget = &mut self.widget;
        events::with_event_map(map, || widget.poll())
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

