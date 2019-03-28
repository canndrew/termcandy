#![feature(never_type)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(specialization)]

/// Create a CSI-introduced sequence.
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

/*
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
*/

/*
#[macro_export]
macro_rules! select_widget (
    (SELECT_WIDGET_DECOMPOSE ($p:pat = $e:expr => $b:expr,$($rest:tt)*) INTO $es:tt THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_DECOMPOSE ($($rest)*) INTO (&mut $e, $es) THEN ($p => $b, $($stuff)*))
    );
    (SELECT_WIDGET_DECOMPOSE () INTO $es:tt THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_REVERSE () FROM $es THEN ($($stuff)*))
    );
    (SELECT_WIDGET_REVERSE $rs:tt FROM ($e:expr, $es:expr) THEN ($($stuff:tt)*)) => (
        {
            let wowzers = stringify!($rs FROM ($e, $es) THEN ($($stuff)*));
            select_widget!(SELECT_WIDGET_REVERSE ($e, $rs) FROM $es THEN ($($stuff)*))
        }
    );
    (SELECT_WIDGET_REVERSE $rs:tt FROM () THEN ($($p:pat => $b:expr,)*)) => ({
        let widgets = $rs;

        loop {
            {
                let tail = &mut widgets;
                $(
                    let (mut head, ref mut tail) = *tail;
                    match head.poll()? {
                        Async::Ready($p) => break $b,
                        Async::NotReady => (),
                    }
                )*
            }
            yield unsafe {
                termcandy::widget::forge_lifetime(Box::new(|surface| {
                    let tail = &mut widgets;
                    $(
                        let _ = stringify!($pat);
                        let (mut head, ref mut tail) = *tail;
                        head.draw(surface);
                    )*
                }))
            }
        }
    });
    ($($all:tt)*) => {
        select_widget!(SELECT_WIDGET_DECOMPOSE ($($all)*) INTO () THEN ())
    }
);
*/

/*
#[macro_export]
macro_rules! select_widget (
    (SELECT_WIDGET_DECOMPOSE ($p:pat = $e:expr => $b:expr,$($rest:tt)*) INTO ($($es:tt)*) THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_DECOMPOSE ($($rest)*) INTO (&mut $e, $($es)*) THEN ($p => $b, $($stuff)*))
    );
    (SELECT_WIDGET_DECOMPOSE () INTO ($($es:tt)*) THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_REVERSE () FROM ($($es)*) THEN ($($stuff)*))
    );
    (ZONGO $($all:tt)*) => {
        select_widget!(SELECT_WIDGET_DECOMPOSE ($($all)*) INTO () THEN ())
    };
    (SELECT_WIDGET_REVERSE $rs:tt FROM ($e:expr, $es:expr) THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_REVERSE_2 ($e, $rs) FROM ($es) THEN ($($stuff)*))
    );
    (SELECT_WIDGET_REVERSE_2 $rs:tt FROM ($e:expr, $es:expr) THEN ($($stuff:tt)*)) => (
        select_widget!(SELECT_WIDGET_REVERSE_3 ($e, $rs) FROM ($es) THEN ($($stuff)*))
    );
);
*/


mod io;
pub mod graphics;
mod screen;
mod run;
mod input;
mod layout;
pub mod events;
#[doc(hidden)]
pub mod widget;
pub use run::run;
pub use widget::{widget, Widget, DrawAs, FutureExt};
pub use widget_macro::{select_widget, await_widget};
pub use layout::VecJoin;

