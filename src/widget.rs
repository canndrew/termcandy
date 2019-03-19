use std::ops::{Generator, GeneratorState};
use std::marker::PhantomPinned;
use futures::{Async, Future};
use crate::graphics::SurfaceMut;
pub use widget_macro::widget;

pub trait Widget: Future {
    fn draw(&self, surface: SurfaceMut);
}

#[doc(hidden)]
pub struct GenWidget<G> {
    gen: G,
    pub drawer: Option<Box<dyn Fn(SurfaceMut) + 'static>>,
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
pub fn nil_drawer(_: SurfaceMut) {}

impl<T, E, G> Future for GenWidget<G>
where
    G: Generator<Yield = Box<dyn Fn(SurfaceMut) + 'static>, Return = Result<T, E>>
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Result<Async<T>, E> {
        drop(self.drawer.take());
        match unsafe { self.gen.resume() } {
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
    G: Generator<Yield = Box<dyn Fn(SurfaceMut) + 'static>, Return = Result<T, E>>
{
    fn draw(&self, surface: SurfaceMut) {
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
pub unsafe fn forge_lifetime<'a>(b: Box<Fn(SurfaceMut) + 'a>) -> Box<Fn(SurfaceMut) + 'static> {
    std::mem::transmute(b)
}

pub trait FutureExt: Future {
    fn draw_as<D>(self, drawer: D) -> DrawAs<Self, D>
    where
        Self: Sized,
        D: Fn(SurfaceMut),
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
    D: Fn(SurfaceMut),
{
    fn draw(&self, surface: SurfaceMut) {
        (self.drawer)(surface)
    }
}

impl<F> FutureExt for F
where
    F: Future
{}

