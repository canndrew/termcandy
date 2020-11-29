use super::*;

use tokio::signal::unix::{signal, Signal, SignalKind};

use crate::terminal::{AlternateScreen, MouseTerminal, NonBlockingStdout, RawMode, Blocking};
use crate::graphics::{Color, Style, Surface, UnderlineKind};
use crate::widget::Widget;

task_local! {
    static SCREEN_SIZE: std::cell::Cell<(u16, u16)>; // = std::cell::Cell::new((0, 0));
}

pub async fn with_screen<F, U>(stdout: NonBlockingStdout, func: F) -> io::Result<U::Output>
where
    F: FnOnce(Screen) -> U,
    U: Future,
{
    let (w, h) = termion::terminal_size()?;
    let screen = Screen::new(stdout, w, h).await?;
    Ok(SCREEN_SIZE.scope(std::cell::Cell::new((w, h)), func(screen)).await)
}

struct Buffers {
    front_buffer: Surface,
    back_buffer: Surface,
    writing: Vec<u8>,
    amount_written: usize,
    damaged: bool,
    cursor_x: u16,
    cursor_y: u16,
    current_style: Style,
}

impl Buffers {
    fn resize(&mut self, w: u16, h: u16) {
        self.front_buffer = Surface::blank(w, h);
        self.back_buffer = Surface::blank(w, h);
        self.writing.clear();
        self.writing.reserve(w as usize * h as usize * 2);
        self.damage();
    }

    fn damage(&mut self) {
        self.damaged = true;
        self.writing.clear();
        write!(&mut self.writing, "{}", termion::cursor::Goto(1, 1)).unwrap();
        self.amount_written = 0;
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn draw_widget<W>(&mut self, widget: &W)
    where
        W: Widget
    {
        let mut surface = self.back_buffer.as_mut();
        surface.clear();
        widget.draw(&mut surface);
    }

    fn move_cursor(&mut self, x: u16, y: u16) {
        // TODO: This gets called a lot. Need better optimizations here for moving cursor position
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

#[pin_project]
pub struct Screen {
    #[pin]
    inner: AlternateScreen<MouseTerminal<RawMode<NonBlockingStdout>>>,
    #[pin]
    sigwinch: Signal,
    buffers: Buffers,
}

impl Screen {
    pub async fn new(stdout: NonBlockingStdout, w: u16, h: u16) -> io::Result<Screen> {
        let stdout = RawMode::new(stdout)?;
        let stdout = MouseTerminal::new(stdout).await?;
        let stdout = AlternateScreen::new(stdout).await?;
        let sigwinch = signal(SignalKind::window_change())?;
        let mut writing = Vec::new();
        let _ = write!(writing, "{}", termion::cursor::Hide);
        let buffers = Buffers {
            front_buffer: Surface::blank(w, h),
            back_buffer: Surface::blank(w, h),
            writing: writing,
            damaged: true,
            amount_written: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_style: Style::default(),
        };
        Ok(Screen {
            inner: stdout,
            sigwinch,
            buffers,
        })
    }

    pub fn poll_for_resizes(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<(u16, u16)>> {
        let this = self.project();
        if let Poll::Ready(Some(())) = this.sigwinch.poll_next(cx) {
            let (w, h) = match termion::terminal_size() {
                Err(err) => return Poll::Ready(Err(err)),
                Ok(size) => size,
            };
            SCREEN_SIZE.with(|screen_size| screen_size.set((w, h)));
            this.buffers.resize(w, h);
            return Poll::Ready(Ok((w, h)));
        }
        Poll::Pending
    }

    /*
    pub fn damage(&mut self) {
        self.buffers.damage();
    }
    */

    pub fn draw_widget<W>(&mut self, widget: &W)
    where
        W: Widget
    {
        self.buffers.draw_widget(widget)
    }

    pub fn flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        match Screen::flush_front(this.inner.as_mut(), cx, this.buffers) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(())) => (),
        };
        this.buffers.swap_buffers();
        Screen::flush_front(this.inner.as_mut(), cx, this.buffers)
    }

    fn flush_front(
        mut inner: Pin<&mut AlternateScreen<MouseTerminal<RawMode<NonBlockingStdout>>>>,
        cx: &mut Context<'_>,
        buffers: &mut Buffers,
    ) -> Poll<io::Result<()>> {
        loop {
            if buffers.amount_written == buffers.writing.len() {
                buffers.writing.clear();
                buffers.amount_written = 0;
                return Poll::Ready(Ok(()));
            }
            trace!("screen: {:?}", &buffers.writing[buffers.amount_written..]);
            match inner.as_mut().poll_write(cx, &buffers.writing[buffers.amount_written..]) {
                Poll::Ready(Ok(n)) => buffers.amount_written += n,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        if let Ok(blocking) = Blocking::new() {
            let _ = Write::write_all(&mut self.inner, termion::cursor::Show.as_ref());
            drop(blocking);
        }
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

