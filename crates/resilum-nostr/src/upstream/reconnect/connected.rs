//! The liveness flag every `Upstream` handle reads, raised and cleared as one.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Tells every `Upstream` handle the connection is usable, and stops telling
/// them on every way the connected branch can end — including the task being
/// aborted or dropped mid-connection, cases a plain `store(false)` placed
/// after the work would never run for.
pub(super) struct Connected(Arc<AtomicBool>);

impl Connected {
    pub(super) fn raise(flag: Arc<AtomicBool>) -> Self {
        flag.store(true, Ordering::Relaxed);
        Self(flag)
    }
}

impl Drop for Connected {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}
