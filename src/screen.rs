use std::io::Write;
use std::io;
use unicode_width::UnicodeWidthChar;
use futures::{Async, Future, Stream, IntoFuture};
use libc::SIGWINCH;
use tokio_signal::unix::Signal;
use tokio::reactor::PollEvented2;
use tokio::io::AsyncWrite;
use futures::task_local;

use crate::io::{force_write, AlternateScreen, MouseTerminal, NonBlockingStdout, RawMode};
use crate::graphics::{Color, Style, Surface, UnderlineKind};
use crate::widget::Widget;

task_local!(static SCREEN_SIZE: std::cell::Cell<(u16, u16)> = std::cell::Cell::new((0, 0)));

pub struct Screen {
    inner: PollEvented2<AlternateScreen<MouseTerminal<RawMode<NonBlockingStdout>>>>,
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
    pub fn new(stdout: NonBlockingStdout, w: u16, h: u16)
        -> impl Future<Item=Screen, Error=io::Error> + 'static
    {
        RawMode::new(stdout)
        .into_future()
        .and_then(move |stdout| {
            MouseTerminal::new(stdout)
            .and_then(move |stdout| {
                AlternateScreen::new(stdout)
                .join(Signal::new(SIGWINCH))
                .map(move |(inner, signal)| {
                    SCREEN_SIZE.with(|screen_size| screen_size.set((w, h)));
                    let mut writing = Vec::new();
                    let _ = write!(writing, "{}", termion::cursor::Hide);
                    Screen {
                        inner: PollEvented2::new(inner),
                        front_buffer: Surface::blank(w, h),
                        back_buffer: Surface::blank(w, h),
                        sigwinch: signal,
                        writing: writing,
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
            SCREEN_SIZE.with(|screen_size| screen_size.set((w, h)));
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
        let mut surface = self.back_buffer.as_mut();
        surface.clear();
        widget.draw(&mut surface);
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
                self.writing.clear();
                self.amount_written = 0;
                return Ok(Async::Ready(()));
            }
            match self.inner.poll_write(&self.writing[self.amount_written..])? {
                Async::Ready(n) => self.amount_written += n,
                Async::NotReady => return Ok(Async::NotReady),
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
            match style.fg {
                Color::Default => (),
                Color::Colors16 { code, bright: true } => {
                    write!(&mut self.writing, "\x1b[9{};1m", code as u32).unwrap();
                },
                Color::Colors16 { code, bright: false } => {
                    write!(&mut self.writing, "\x1b[3{};1m", code as u32).unwrap();
                },
                Color::Colors256(x) => {
                    write!(&mut self.writing, "\x1b[38;5;{}m", x).unwrap();
                },
                Color::Rgb { r, g, b } => {
                    write!(&mut self.writing, "\x1b[38;2;{};{};{}m", r, g, b).unwrap();
                },
            }
            match style.bg {
                Color::Default => (),
                Color::Colors16 { code, bright: true } => {
                    write!(&mut self.writing, "\x1b[10{};1m", code as u32).unwrap();
                },
                Color::Colors16 { code, bright: false } => {
                    write!(&mut self.writing, "\x1b[4{};1m", code as u32).unwrap();
                },
                Color::Colors256(x) => {
                    write!(&mut self.writing, "\x1b[48;5;{}m", x).unwrap();
                },
                Color::Rgb { r, g, b } => {
                    write!(&mut self.writing, "\x1b[48;2;{};{};{}m", r, g, b).unwrap();
                },
            }
            if style.attrs.bold {
                write!(&mut self.writing, "\x1b[1m").unwrap();
            }
            if style.attrs.faint {
                write!(&mut self.writing, "\x1b[2m").unwrap();
            }
            if style.attrs.italic {
                write!(&mut self.writing, "\x1b[3m").unwrap();
            }
            if style.attrs.blink {
                write!(&mut self.writing, "\x1b[5m").unwrap();
            }
            if style.attrs.strikethrough {
                write!(&mut self.writing, "\x1b[9m").unwrap();
            }
            if style.attrs.overlined {
                write!(&mut self.writing, "\x1b[53m").unwrap();
            }
            if let Some(underline) = style.attrs.underline {
                match underline.kind {
                    UnderlineKind::Single => {
                        write!(&mut self.writing, "\x1b[4m").unwrap();
                    },
                    UnderlineKind::Double => {
                        write!(&mut self.writing, "\x1b[4:2m").unwrap();
                    },
                    UnderlineKind::Wavy => {
                        write!(&mut self.writing, "\x1b[4:3m").unwrap();
                    },
                }
                match underline.color {
                    Color::Default => (),
                    Color::Colors16 { code, bright } => {
                        let x = (code as u32) + if bright { 8 } else { 0 };
                        write!(&mut self.writing, "\x1b[58;5;{}m", x).unwrap();
                    },
                    Color::Colors256(x) => {
                        write!(&mut self.writing, "\x1b[58;5;{}m", x).unwrap();
                    },
                    Color::Rgb { r, g, b } => {
                        write!(&mut self.writing, "\x1b[58;2;{};{};{}m", r, g, b).unwrap();
                    },
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

impl Drop for Screen {
    fn drop(&mut self) {
        let mut v = Vec::new();
        let _ = write!(v, "{}", termion::cursor::Show);
        force_write(&mut self.inner, v);
    }
}

pub(crate) fn with_screen_size<F, R>(w: u16, h: u16, func: F) -> R
where
    F: FnOnce() -> R,
{
    let (old_w, old_h) = SCREEN_SIZE.with(|screen_size| {
        let (old_w, old_h) = screen_size.get();
        screen_size.set((w, h));
        (old_w, old_h)
    });
    let ret = func();
    SCREEN_SIZE.with(|screen_size| {
        screen_size.set((old_w, old_h));
    });
    ret
}

/// Get the size of the screen as seen by the current widget.
///
/// # Note
///
/// Nesting widgets inside each other (using `Widget::resize`) will cause the nested widget to only
/// see the size of the region of the screen that it's been allocated.
pub fn screen_size() -> (u16, u16) {
    SCREEN_SIZE.with(|screen_size| {
        screen_size.get()
    })
}

