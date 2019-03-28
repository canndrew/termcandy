use std::time::{Duration, Instant};
use futures::{Future, Async};
use std::thread;
use tokio::io::AsyncWrite;

// Blocks, trying to forcefully write something to the output.
// Used in Drop implementation when trying to put the terminal back into a sane state.
pub fn force_write<W: AsyncWrite, S: AsRef<[u8]>>(w: &mut W, s: S) {
    let deadline = Instant::now() + Duration::from_millis(200);
    let mut write_all = tokio::io::write_all(w, s.as_ref());
    loop {
        if let Ok(Async::NotReady) = write_all.poll() {
            if deadline > Instant::now() {
                thread::yield_now();
                continue;
            }
        }
        break;
    }
}

