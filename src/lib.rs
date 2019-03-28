#![feature(never_type)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(specialization)]

/// Create a CSI-introduced sequence.
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

mod io;
pub mod graphics;
mod screen;
mod run;
mod input;
pub mod events;
#[doc(hidden)]
pub mod widget;
pub use run::{RunError, run};
pub use widget::{Widget, FutureExt};
pub use termcandy_macros::{widget, select_widget, await_widget};
pub use screen::screen_size;

