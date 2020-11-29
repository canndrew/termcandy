use super::*;

pub use pin_utils::pin_mut;
pub use std::future::Future;
pub use std::task::{Poll, Context};
pub use std::pin::Pin;

use std::ops::{Generator, GeneratorState};
use std::marker::PhantomPinned;

use crate::graphics::SurfaceMut;
use crate::widget::Widget;

pub unsafe fn forge_drawer_lifetime<'a>(b: Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'a>)
    -> Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>
{
    std::mem::transmute(b)
}

// NOTE: This shouldn't be necessary. But due to Rust's buggy/incomplete lifetime-parameter
// inference there's no way to avoid it. At some point both this function and the call to it should
// be able to just be removed.
pub unsafe fn forge_generator_lifetime<'a, 'b, Y, R>(
    generator: Pin<Box<dyn Generator<&'a mut Context<'b>, Yield = Y, Return = R>>>,
) -> Pin<Box<dyn for<'c, 'd> Generator<&'c mut Context<'d>, Yield = Y, Return = R>>> {
    std::mem::transmute(generator)
}

pub trait WidgetOrFuture: Future {
    fn widget_or_future_draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>);
}

impl<W: Future> WidgetOrFuture for W {
    default fn widget_or_future_draw<'s, 'm>(&self, _surface: &'m mut SurfaceMut<'s>) {
    }
}

impl<W: Widget> WidgetOrFuture for W {
    fn widget_or_future_draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        self.draw(surface)
    }
}

#[pin_project]
pub struct GenWidget<G> {
    #[pin]
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

pub fn nil_drawer<'s, 'm>(_: &'m mut SurfaceMut<'s>) {}

impl<T, G> Future for GenWidget<G>
where
    G: for<'c, 'p> Generator<
        &'c mut Context<'p>,
        Yield = Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>,
        Return = T,
    >,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        {
            let ass = self.as_ref().get_ref();
            trace!("in GenWidget::poll(self == {:?})", ass as *const _);
        }

        let this = self.project();
        if let Some(prev_drawer) = this.drawer.take() {
            trace!("dropping prev drawer: {:?}", &*prev_drawer as *const (dyn for<'x, 'y> Fn(&'y mut SurfaceMut<'x>) + 'static));
            drop(prev_drawer);
        }

        match this.gen.resume(cx) {
            GeneratorState::Yielded(drawer) => {
                trace!("got drawer: {:?}", &*drawer as *const (dyn for<'x, 'y> Fn(&'y mut SurfaceMut<'x>) + 'static));
                *this.drawer = Some(drawer);
                Poll::Pending
            },
            GeneratorState::Complete(val) => {
                Poll::Ready(val)
            },
        }
    }
}

impl<T, G> Widget for GenWidget<G>
where
    G: for<'c, 'p> Generator<
        &'c mut Context<'p>,
        Yield = Box<dyn for<'s, 'm> Fn(&'m mut SurfaceMut<'s>) + 'static>,
        Return = T,
    >,
{
    fn draw<'s, 'm>(&self, surface: &'m mut SurfaceMut<'s>) {
        {
            trace!("in GenWidget::draw(self == {:?})", self as *const _);
        }

        if let Some(drawer) = &self.drawer {
            trace!("calling drawer: {:?}", &**drawer as *const (dyn for<'x, 'y> Fn(&'y mut SurfaceMut<'x>) + 'static));
            drawer(surface)
        }
    }
}

impl<G> Drop for GenWidget<G> {
    fn drop(&mut self) {
        // need to make sure this gets dropped first.
        // since it can reference gen
        if let Some(drawer) = self.drawer.take() {
            trace!("dropping drawer: {:?}", &*drawer as *const _);
            drop(drawer);
        }
    }
}

