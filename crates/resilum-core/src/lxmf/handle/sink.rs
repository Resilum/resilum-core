//! The processor's end of the event queue.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use super::{MAX_QUEUED_EVENTS, Overflow, overflow};

/// Above this many remembered peers the map is swept of the ones whose
/// announce has already been read. Twice the queue bound, so a sweep always
/// has at least that many entries to reclaim and cannot become per-push work.
const MAX_REMEMBERED_PEERS: usize = 2 * MAX_QUEUED_EVENTS;

/// One entry of the queue. An announce is held indirectly so a newer one from
/// the same peer can replace it where it already sits instead of queueing
/// behind it.
pub(in crate::lxmf) enum Queued {
    Json(String),
    Announce(Arc<Slot>),
}

impl Queued {
    pub(super) fn into_json(self) -> Option<String> {
        match self {
            Self::Json(json) => Some(json),
            Self::Announce(slot) => slot.take(),
        }
    }
}

/// One peer's place in the queue, full exactly while that place is occupied.
/// Reading and writing it under the same lock is what lets the sink tell a
/// supersede from a first arrival without racing the reader.
pub(in crate::lxmf) struct Slot(Mutex<Option<String>>);

impl Slot {
    fn empty() -> Arc<Self> {
        Arc::new(Self(Mutex::new(None)))
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Replaces a pending announce in place, or hands `json` back when this
    /// peer holds no place in the queue and one has to be taken.
    fn supersede(&self, json: String) -> Option<String> {
        let mut held = self.held();
        if held.is_none() {
            return Some(json);
        }
        *held = Some(json);
        None
    }

    fn fill(&self, json: String) {
        *self.held() = Some(json);
    }

    fn take(&self) -> Option<String> {
        self.held().take()
    }

    fn is_pending(&self) -> bool {
        self.held().is_some()
    }
}

/// Every push is non-blocking, which is what lets it run under the core lock.
pub(in crate::lxmf) struct EventSink {
    events: mpsc::Sender<Queued>,
    depth: Arc<AtomicUsize>,
    announces: HashMap<[u8; 16], Arc<Slot>>,
    dropped: u64,
}

impl EventSink {
    pub(in crate::lxmf) fn push(&mut self, json: String) {
        if self.reserve() {
            self.emit(Queued::Json(json));
        }
    }

    /// A peer's latest announce, superseding whatever it had queued.
    ///
    /// A supersede costs no slot, so it is allowed even on a full queue: what
    /// it loses is an announce the next one replaces anyway, and what it keeps
    /// are the delivery updates that would have been crowded out.
    pub(in crate::lxmf) fn push_announce(&mut self, source: [u8; 16], json: String) {
        self.forget_read_peers();
        let slot = Arc::clone(self.announces.entry(source).or_insert_with(Slot::empty));
        let Some(json) = slot.supersede(json) else {
            return;
        };
        if !self.reserve() {
            return;
        }
        slot.fill(json);
        if !self.emit(Queued::Announce(Arc::clone(&slot))) {
            slot.take();
        }
    }

    fn reserve(&mut self) -> bool {
        if self.depth.load(Ordering::Relaxed) >= MAX_QUEUED_EVENTS {
            self.dropped += 1;
            return false;
        }
        if self.dropped > 0 {
            let lost = std::mem::take(&mut self.dropped);
            self.emit(Queued::Json(overflow(Overflow::Delivery, lost)));
        }
        true
    }

    fn emit(&self, queued: Queued) -> bool {
        if self.events.send(queued).is_err() {
            return false;
        }
        self.depth.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// An emptied slot is one that has been read, so nothing points at it any
    /// more and the peer can be remembered again from scratch.
    fn forget_read_peers(&mut self) {
        if self.announces.len() <= MAX_REMEMBERED_PEERS {
            return;
        }
        self.announces.retain(|_, slot| slot.is_pending());
    }
}

pub(super) fn new(events: mpsc::Sender<Queued>, depth: Arc<AtomicUsize>) -> EventSink {
    EventSink {
        events,
        depth,
        announces: HashMap::new(),
        dropped: 0,
    }
}

#[cfg(test)]
mod tests;
