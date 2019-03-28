# `termcandy`

`termcandy` is a library for writing terminal user interfaces using
imperative-style code. This means you don't have to structure your program
around an event loop, just write natural-looking control flow and let macro
magic do the rest.

## Example

This program will draw a blue ball bouncing around the screen until the user
presses escape.

```rust
#![feature(proc_macro_hygiene)]
#![feature(never_type)]
#![feature(generators)]
#![feature(label_break_value)]

use std::time::{Instant, Duration};
use termcandy::{widget, select_widget};
use termcandy::events::{self, Key};
use termcandy::graphics::{Style, Color, Attrs};
use tokio::timer;
use futures::Future;

#[widget]
fn bouncing_ball() -> Result<(), failure::Error> {
    let mut pos_x = 0;
    let mut pos_y = 0;
    let mut vel_x = 1;
    let mut vel_y = 1;
    let mut next_instant = Instant::now();
    let style = Style { fg: Color::blue(), bg: Color::default(), attrs: Attrs::bold() };
    loop {
        select_widget! {
            () = timer::Delay::new(next_instant) => {
                next_instant += Duration::from_millis(100);
                let (w, h) = termcandy::screen_size();
                if pos_x <= 0 { vel_x = 1 };
                if pos_x >= w as i16 { vel_x = -1 };
                if pos_y <= 0 { vel_y = 1 };
                if pos_y >= h as i16 { vel_y = -1 };
                pos_x += vel_x;
                pos_y += vel_y;
            },
            () = events::key(Key::Esc) => return Ok(()),
            never = widget::draw(|surface| {
                surface.print("●", pos_x, pos_y, style)
            }) => never,
        }
    }
}

fn main() {
    tokio::runtime::current_thread::block_on_all(termcandy::run(bouncing_ball())).expect("oh no!")
}
```

We could also reuse the above code to make 4 balls bounce around inside their own boxes:

```rust
use termcandy::Widget;
use termcandy::graphics::Rect;

#[widget]
fn four_bouncing_balls() -> Result<(), failure::Error> {
    let top_left = bouncing_ball().resize(|w, h| {
        Rect { x0: 0, x1: w as i16 / 2, y0: 0, y1: h as i16 / 2 }
    });
    let top_right = bouncing_ball().resize(|w, h| {
        Rect { x0: w as i16 / 2, x1: w as i16, y0: 0, y1: h as i16 / 2 }
    });
    let bottom_left = bouncing_ball().resize(|w, h| {
        Rect { x0: 0, x1: w as i16 / 2, y0: h as i16 / 2, y1: h as i16 }
    });
    let bottom_right = bouncing_ball().resize(|w, h| {
        Rect { x0: w as i16 / 2, x1: w as i16, y0: h as i16 / 2, y1: h as i16 }
    });
    select_widget! {
        () = top_left => (),
        () = top_right => (),
        () = bottom_left => (),
        () = bottom_right => (),
        never = widget::draw(|surface| {
            surface.draw_v_line(0, surface.height() as i16 - 1, 0);
            surface.draw_v_line(0, surface.height() as i16 - 1, surface.width() as i16 / 2);
            surface.draw_v_line(0, surface.height() as i16 - 1, surface.width() as i16 - 1);
            surface.draw_h_line(0, surface.width() as i16 - 1, 0);
            surface.draw_h_line(0, surface.width() as i16 - 1, surface.height() as i16 / 2);
            surface.draw_h_line(0, surface.width() as i16 - 1, surface.height() as i16 - 1);
        }) => never
    }
    Ok(())
}
```

