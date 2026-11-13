//! Lets background work wake the UI. The GUI only redraws on input or when asked to, so
//! anything that delivers results from another thread (journal snapshots, API responses,
//! upload progress) calls [`wake`] after sending them. The terminal UI polls on its own
//! loop and never installs a waker, so there this is a no-op.

use std::sync::OnceLock;

type Waker = Box<dyn Fn() + Send + Sync>;

static WAKER: OnceLock<Waker> = OnceLock::new();

/// Installs the function that wakes the UI. Only the first call takes effect.
pub fn install(waker: impl Fn() + Send + Sync + 'static) {
    let _ = WAKER.set(Box::new(waker));
}

/// Asks the UI to run another frame soon, from any thread.
pub fn wake() {
    if let Some(w) = WAKER.get() {
        w();
    }
}
