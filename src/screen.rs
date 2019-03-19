use std::io::Write;
use std::io;
use unicode_width::UnicodeWidthChar;
use futures::{Async, Future, Stream, IntoFuture};
use libc::SIGWINCH;
use tokio_signal::unix::Signal;

use crate::io::{AlternateScreen, MouseTerminal, NonBlockingStdout, RawMode};
use crate::graphics::{Color, Style, Surface};
use crate::widget::Widget;

type ScreenInner = AlternateScreen<MouseTerminal<RawMode<NonBlockingStdout>>>;

pub struct Screen {
    inner: ScreenInner,
    sigwinch: Signal,
    front_buffer: Surface,
    back_buffer: Surface,
    writing: Vec<u8>,
    amount_written: usize,
    damaged: bool,
    cursor_x: u16,
    cursor_y: u16,
    current_style: Style,
}

impl Screen {
    pub fn new(stdout: NonBlockingStdout, w: u16, h: u16) -> impl Future<Item=Screen, Error=io::Error> + 'static {
        RawMode::new(stdout)
        .into_future()
        .and_then(move |stdout| {
            MouseTerminal::new(stdout)
            .and_then(move |stdout| {
                AlternateScreen::new(stdout)
                .join(Signal::new(SIGWINCH))
                .map(move |(inner, signal)| {
                    Screen {
                        inner: inner,
                        front_buffer: Surface::blank(w, h),
                        back_buffer: Surface::blank(w, h),
                        sigwinch: signal,
                        writing: Vec::new(),
                        damaged: true,
                        amount_written: 0,
                        cursor_x: 0,
                        cursor_y: 0,
                        current_style: Style::default(),
                    }
                })
            })
        })
    }

    pub fn poll_for_resizes(&mut self) -> io::Result<Async<(u16, u16)>> {
        if let Async::Ready(Some(SIGWINCH)) = self.sigwinch.poll()? {
            let (w, h) = termion::terminal_size()?;
            self.front_buffer = Surface::blank(w, h);
            self.back_buffer = Surface::blank(w, h);
            self.writing = Vec::with_capacity(w as usize * h as usize * 2);
            self.damage();
            return Ok(Async::Ready((w, h)));
        }
        Ok(Async::NotReady)
    }

    pub fn damage(&mut self) {
        self.damaged = true;
        self.writing.clear();
        write!(&mut self.writing, "{}", termion::cursor::Goto(1, 1)).unwrap();
        self.amount_written = 0;
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn draw_widget<W>(&mut self, widget: &W)
    where
        W: Widget
    {
        widget.draw(self.back_buffer.as_mut())
    }

    pub fn flush(&mut self) -> io::Result<Async<()>> {
        match self.flush_front()? {
            Async::NotReady => return Ok(Async::NotReady),
            Async::Ready(()) => (),
        };
        self.swap_buffers();
        self.flush_front()
    }

    fn flush_front(&mut self) -> io::Result<Async<()>> {
        loop {
            if self.amount_written == self.writing.len() {
                return Ok(Async::Ready(()));
            }
            match self.inner.write(&self.writing[self.amount_written..]) {
                Ok(n) => self.amount_written += n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(Async::NotReady);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn move_cursor(&mut self, x: u16, y: u16) {
        // TODO: better optimizations here for moving cursor position
        if x != self.cursor_x || y != self.cursor_y {
            write!(&mut self.writing, "{}", termion::cursor::Goto(x + 1, y + 1)).unwrap();
            self.cursor_x = x;
            self.cursor_y = y;
        }
    }

    fn set_style(&mut self, style: Style) {
        if self.current_style != style {
            write!(&mut self.writing, "\x1b[0m").unwrap();
            if let Color::Named(x, bold) = style.fg {
                if bold {
                    write!(&mut self.writing, "\x1b[3{};1m", x).unwrap();
                } else {
                    write!(&mut self.writing, "\x1b[3{}m", x).unwrap();
                }
            }
            if let Color::Named(x, bold) = style.bg {
                if bold {
                    write!(&mut self.writing, "\x1b[4{};1m", x).unwrap();
                } else {
                    write!(&mut self.writing, "\x1b[4{}m", x).unwrap();
                }
            }
            self.current_style = style;
        }
    }

    fn swap_buffers(&mut self) {
        debug_assert_eq!(self.front_buffer.width(), self.back_buffer.width());
        debug_assert_eq!(self.front_buffer.height(), self.back_buffer.height());

        let w = self.front_buffer.width();
        let h = self.front_buffer.height();
        let mut x = 0;
        let mut y = 0;
        while y < h {
            let c = self.back_buffer.cell(x, y).c;
            let char_width = c.width().unwrap_or(0) as u16;

            if (self.damaged || self.back_buffer.cell(x, y) != self.front_buffer.cell(x, y)) &&
                char_width != 0
            {
                self.move_cursor(x, y);

                let style = self.back_buffer.cell(x, y).style;
                self.set_style(style);

                write!(&mut self.writing, "{}", c).unwrap();
                *self.front_buffer.cell_mut(x, y) = *self.back_buffer.cell(x, y);
                for i in 1..char_width {
                    if x + i == w {
                        break;
                    }
                    self.front_buffer.cell_mut(x + i, y).c = '\0';
                }
                self.cursor_x += char_width;
                if self.cursor_x >= w {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                }
            }
            x += 1;
            if x == w {
                x = 0;
                y += 1;
            }
        }
        self.damaged = false;
    }
}


