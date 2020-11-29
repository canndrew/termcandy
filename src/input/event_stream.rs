//! Functions and types for capturing user-input events.

use super::*;

pub use termion::event::{Key, Event, MouseEvent, MouseButton};

#[pin_project]
pub struct EventStream {
    #[pin]
    event_watcher_opt: Option<EventWatcher>,
}

impl Stream for EventStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Event>> {
        let this = self.project();
        match this.event_watcher_opt.as_pin_mut() {
            Some(event_watcher) => event_watcher.poll_next(cx),
            None => Poll::Ready(None),
        }
    }
}

impl FusedStream for EventStream {
    fn is_terminated(&self) -> bool {
        match self.event_watcher_opt.as_ref() {
            Some(event_watcher) => event_watcher.is_terminated(),
            None => true,
        }
    }
}

pub fn event_stream() -> EventStream {
    EventStream {
        event_watcher_opt: EventWatcher::new(),
    }
}

pub async fn matching<T>(mut func: impl FnMut(Event) -> Option<T>) -> T {
    let mut event_stream = event_stream();
    loop {
        let event = event_stream.select_next_some().await;
        match func(event) {
            Some(value) => return value,
            None => (),
        }
    }
}

/// The given key.
pub async fn key(key: Key) {
    matching(|event| match event {
        Event::Key(got) if got == key => Some(()),
        _ => None,
    }).await
}

/// The given escape sequence, if the escape sequence wasn't parsed by termion.
pub async fn unsupported<'a>(bytes: &'a [u8]) {
    matching(|event| match event {
        Event::Unsupported(ref v) if &v[..] == bytes => Some(()),
        _ => None,
    }).await
}

/// Mouse left button clicked.
pub async fn left_click() -> (u16, u16) {
    matching(|event| match event {
        Event::Mouse(MouseEvent::Press(MouseButton::Left, x, y)) => Some((x - 1, y - 1)),
        _ => None,
    }).await
}

/// Mouse right button clicked.
pub async fn right_click() -> (u16, u16) {
    matching(|event| match event {
        Event::Mouse(MouseEvent::Press(MouseButton::Right, x, y)) => Some((x - 1, y - 1)),
        _ => None,
    }).await
}

/// Mouse clicked.
pub async fn click(button: MouseButton) -> (u16, u16) {
    matching(|event| match event {
        Event::Mouse(MouseEvent::Press(got_button, x, y))
            if button == got_button
            => Some((x - 1, y - 1)),

        _ => None,
    }).await
}

/// Mouse button held and dragged.
pub async fn hold() -> (u16, u16) {
    matching(|event| match event {
        Event::Mouse(MouseEvent::Hold(x, y)) => Some((x - 1, y - 1)),
        _ => None,
    }).await
}

/// Mouse button released.
pub async fn release() -> (u16, u16) {
    matching(|event| match event {
        Event::Mouse(MouseEvent::Release(x, y)) => Some((x - 1, y - 1)),
        _ => None,
    }).await
}

/// Any mouse event.
pub async fn any_mouse_event() -> MouseEvent {
    matching(|event| match event {
        Event::Mouse(mouse_event) => Some(mouse_event),
        _ => None,
    }).await
}

/// Any keystroke.
pub async fn any_key() -> Key {
    matching(|event| match event {
        Event::Key(key) => Some(key),
        _ => None,
    }).await
}

/// Any unsupported event, returned as a vector of bytes containing the unrecognized escape
/// sequence.
pub async fn any_unsupported() -> Vec<u8> {
    matching(|event| match event {
        Event::Unsupported(v) => Some(v),
        _ => None,
    }).await
}

/// Any terminal user-input event
pub async fn any() -> Event {
    let mut event_stream = event_stream();
    event_stream.select_next_some().await
}
