use std::ops::{Generator, GeneratorState};
use std::marker::PhantomPinned;
use futures::{Async, Future};
use crate::graphics::SurfaceMut;
pub use widget_macro::widget;
use crate::events;
use std::pin::Pin;
use termion::event::{MouseEvent, Event};

pub trait Widget: Future {
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>);
    fn decorate<F>(self, draw: F) -> Decorate<F, Self>
    where
        Self: Sized,
        F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>, &dyn for<'ss, 'mm> Fn(&'mm mut SurfaceMut<'ss>)),
    {
        Decorate {
            draw: draw,
            widget: self,
        }
    }
    fn background(self) -> Background<Self>
    where
        Self: Sized,
    {
        Background { widget: self }
    }
    fn map_mouse<M>(self, map: M) -> MapMouse<Self, M>
    where
        Self: Sized,
        M: Fn(u16, u16) -> Option<(u16, u16)> + Sync + Send,
    {
        MapMouse { widget: self, map }
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

pub trait FutureExt: Future {
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

pub struct Decorate<F, W> {
    draw: F,
    widget: W,
}

impl<F, W> Future for Decorate<F, W>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>, &dyn for<'ss, 'mm> Fn(&'mm mut SurfaceMut<'ss>)),
    W: Future,
{
    type Item = W::Item;
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<W::Item>, W::Error> {
        self.widget.poll()
    }
}

impl<F, W> Widget for Decorate<F, W>
where
    F: for<'s, 'm> Fn(&'m mut SurfaceMut<'s>, &dyn for<'ss, 'mm> Fn(&'mm mut SurfaceMut<'ss>)),
    W: Widget,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        (self.draw)(surface, &|surface| self.widget.draw(surface));
    }
}

pub struct Background<W> {
    widget: W,
}

impl<W> Future for Background<W>
where
    W: Future,
{
    type Item = W::Item;
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<W::Item>, W::Error> {
        events::with_event_map(|_event| None, || self.widget.poll())
    }
}

impl<W> Widget for Background<W>
where
    W: Widget,
{
    fn draw<'s, 'm>(&self, _surface: &'m mut SurfaceMut<'s>) {
    }
}

pub struct MapMouse<W, M> {
    widget: W,
    map: M,
}

impl<W, M> Future for MapMouse<W, M>
where
    W: Future,
    M: Fn(u16, u16) -> Option<(u16, u16)> + Sync + Send,
{
    type Item = W::Item;
    type Error = W::Error;

    fn poll(&mut self) -> Result<Async<W::Item>, W::Error> {
        let map = &self.map;
        let widget = &mut self.widget;
        events::with_event_map(
            |event| Some(match event {
                Event::Mouse(mouse_event) => Event::Mouse(match mouse_event {
                    MouseEvent::Press(button, x, y) => {
                        let (x, y) = map(x, y)?;
                        MouseEvent::Press(button, x, y)
                    },
                    MouseEvent::Release(x, y) => {
                        let (x, y) = map(x, y)?;
                        MouseEvent::Release(x, y)
                    },
                    MouseEvent::Hold(x, y) => {
                        let (x, y) = map(x, y)?;
                        MouseEvent::Hold(x, y)
                    },
                }),
                event => event,
            }),
            || widget.poll(),
        )
    }
}

impl<W, M> Widget for MapMouse<W, M>
where
    W: Widget,
    M: Fn(u16, u16) -> Option<(u16, u16)> + Sync + Send,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        self.widget.draw(surface)
    }
}

