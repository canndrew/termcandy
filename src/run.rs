use futures::{Async, Future, IntoFuture};
use std::{io, fmt};
use crate::screen::Screen;
use crate::widget::Widget;
use crate::io::non_blocking_stdio;
use crate::input::Events;
use log::trace;

#[derive(Debug)]
pub enum RunError<E> {
    Widget(E),
    Io(io::Error),
}

impl<E> fmt::Display for RunError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RunError::Widget(e) => write!(formatter, "widget errored: {}", e),
            RunError::Io(e) => write!(formatter, "io error: {}", e),
        }
    }
}

pub fn run<F, W>(f: F) -> impl Future<Item = W::Item, Error = RunError<W::Error>>
where
    F: FnOnce(Events) -> W,
    W: Widget,
{
    termion::terminal_size()
    .into_future()
    .map_err(RunError::Io)
    .and_then(|(w, h)| {
        non_blocking_stdio()
        .into_future()
        .map_err(RunError::Io)
        .and_then(move |(stdin, stdout)| {
            trace!("creating screen with dimensions ({}, {})", w, h);
            Events::new(stdin)
            .into_future()
            .map_err(RunError::Io)
            .and_then(move |events| {
                let widget = f(events);
                Screen::new(stdout, w, h)
                .map_err(RunError::Io)
                .and_then(move |screen| {
                    Run { screen, widget }
                })
            })
        })
    })
}

pub struct Run<W> {
    screen: Screen,
    widget: W,
}

impl<W: Widget> Future for Run<W> {
    type Item = W::Item;
    type Error = RunError<W::Error>;

    fn poll(&mut self) -> Result<Async<W::Item>, RunError<W::Error>> {
        match self.widget.poll().map_err(RunError::Widget)? {
            Async::Ready(val) => return Ok(Async::Ready(val)),
            Async::NotReady => (),
        }

        let _ = self.screen.poll_for_resizes().map_err(RunError::Io)?;
        match self.screen.flush().map_err(RunError::Io)? {
            Async::Ready(()) => (),
            Async::NotReady => return Ok(Async::NotReady),
        }

        self.screen.draw_widget(&self.widget);

        self.screen.flush().map_err(RunError::Io)?;
        Ok(Async::NotReady)
    }
}

