#![feature(never_type)]
#![feature(generators)]
#![feature(generator_trait)]

/// Create a CSI-introduced sequence.
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

#[macro_export]
macro_rules! await_widget (
    ($e:expr) => {{
        use futures::Future;
        use termcandy::Widget;

        let mut widget = $e;
        loop {
            match widget.poll()? {
                futures::Async::Ready(val) => break val,
                futures::Async::NotReady => yield unsafe {
                    termcandy::widget::forge_lifetime(Box::new(|surface| widget.draw(surface)))
                },
            }
        }
    }}
);

mod io;
pub mod graphics;
mod screen;
mod run;
mod input;
#[doc(hidden)]
pub mod widget;
pub use run::run;
pub use widget::{widget, Widget, DrawAs, FutureExt};
pub use input::Events;

