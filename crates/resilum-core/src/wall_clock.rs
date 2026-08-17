//! Unix wall-clock time as seconds with a fraction — the shape both the peer
//! caches and the polled JSON record a sighting in.

use std::time::{SystemTime, UNIX_EPOCH};

/// Zero on a clock set before the epoch, rather than a panic: it reads as a
/// sighting old enough to prune.
pub(crate) fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
