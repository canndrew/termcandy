#![feature(never_type)]
#![feature(exhaustive_patterns)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(specialization)]
#![feature(min_const_generics)]
#![feature(result_flattening)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_slice)]
#![allow(incomplete_features)]

use {
    lazy_static::lazy_static,
    log::trace,
    pin_project::pin_project,
    std::{
        future::Future,
        io::{Read, Write},
        marker::Unpin,
        mem::MaybeUninit,
        os::unix::io::{RawFd, AsRawFd},
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll, Waker},
        time::Duration,
        cmp, env, io, mem, panic,
    },
    futures::{
        stream::FusedStream,
        Stream, StreamExt,
    },
    tokio::{
        io::{
            unix::AsyncFd,
            AsyncRead, AsyncWrite,
        },
        task_local,
    },
    unicode_width::UnicodeWidthChar,
};

pub use {
    crate::{
        run::run,
        widget::{Widget, FutureExt},
        screen::screen_size,
    },
    termcandy_macros::{
        widget, select_widget,
    },
};

mod terminal;
pub mod graphics;
mod screen;
mod run;
pub mod input;
mod cycle_buffer;
pub mod widget;
#[doc(hidden)]
pub mod macros_impl;

#[cfg(debug_assertions)]
#[doc(hidden)]
pub mod logger;

