//! One subscriber's copy of one event.

use crate::queue::Entry;

/// What the queue is keyed by, what a retry is scheduled against, and what an
/// LXMF message in flight stands for. Named fields rather than a pair of
/// 32-byte arrays, which compare positionally and so swap silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Tie {
    pub(super) event_id: [u8; 32],
    pub(super) subscriber: [u8; 32],
}

impl From<&Entry> for Tie {
    fn from(entry: &Entry) -> Self {
        Self {
            event_id: entry.event_id,
            subscriber: entry.subscriber,
        }
    }
}
