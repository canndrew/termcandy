use futures::{Async, future, Future, IntoFuture};
use std::{io, fmt};
use crate::screen::Screen;
use crate::widget::Widget;
use crate::io::non_blocking_stdio;
use log::trace;
use crate::events::EventTaskHandle;

/// Errors returned by `termcandy::run`.
#[derive(Debug)]
pub enum RunError<E> {
    /// The given widget returned an error.
    Widget(E),
    /// IO error using the terminal.
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

/// Create a future that initializes the terminal and runs the given widget.
pub fn run<W>(widget: W) -> impl Future<Item = W::Item, Error = RunError<W::Error>>
where
    W: Widget,
{
    future::lazy(|| {
        termion::terminal_size()
        .into_future()
        .map_err(RunError::Io)
        .and_then(|(w, h)| {
            non_blocking_stdio()
            .into_future()
            .map_err(RunError::Io)
            .and_then(move |(stdin, stdout)| {
                trace!("creating screen with dimensions ({}, {})", w, h);
                EventTaskHandle::new(stdin)
                .into_future()
                .map_err(RunError::Io)
                .and_then(move |event_task_handle| {
                    Screen::new(stdout, w, h)
                    .map_err(RunError::Io)
                    .and_then(move |screen| {
                        Run { screen, widget, _event_task_handle: event_task_handle }
                    })
                })
            })
        })
    })
}

pub struct Run<W> {
    screen: Screen,
    widget: W,
    _event_task_handle: EventTaskHandle,
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

