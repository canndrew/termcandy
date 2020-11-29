use super::*;

use crate::screen::Screen;
use crate::widget::Widget;
use crate::terminal::non_blocking_stdio;

pub async fn run<W>(widget: W) -> io::Result<W::Output>
where
    W: Widget,
{
    let (stdin, stdout) = non_blocking_stdio()?;
    crate::input::with_input_handling(stdin, {
        crate::screen::with_screen(stdout, |screen| {
            Run {
                screen,
                widget,
            }
        })
    }).await.flatten().flatten()
}

#[pin_project]
pub struct Run<W> {
    #[pin]
    screen: Screen,
    #[pin]
    widget: W,
}

impl<W: Widget> Future for Run<W> {
    type Output = io::Result<W::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<W::Output>> {
        let mut this = self.project();
        match this.widget.as_mut().poll(cx) {
            Poll::Ready(val) => return Poll::Ready(Ok(val)),
            Poll::Pending => (),
        }

        // TODO: we don't handle resizing very well.
        // The resize poller could be a separate object, outside of Screen.
        // Also we blank and then flush the screen on a resize, causing flicker when resizing.
        match this.screen.as_mut().poll_for_resizes(cx) {
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(_)) => (),
            Poll::Pending => (),
        }
        match this.screen.as_mut().flush(cx) {
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(())) => (),
        }

        this.screen.draw_widget(this.widget.into_ref().get_ref());

        match this.screen.as_mut().flush(cx) {
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(())) | Poll::Pending => Poll::Pending,
        }
    }
}

