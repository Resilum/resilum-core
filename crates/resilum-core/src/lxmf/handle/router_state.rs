//! What the router reports about itself once per tick — the router runs
//! behind the core lock, so this is how the reading end of the handle learns
//! of it without reaching in.

use std::sync::{Arc, Mutex, MutexGuard};

/// One `Arc` pair shared by the processor (writer) and the handle (reader),
/// built together so neither end can hold a stale clone of the other.
#[derive(Clone)]
pub(in crate::lxmf) struct RouterState {
    outbound: Arc<Mutex<Vec<[u8; 32]>>>,
    propagation_node: Arc<Mutex<Option<[u8; 16]>>>,
}

impl RouterState {
    pub(in crate::lxmf) fn new() -> Self {
        Self {
            outbound: Arc::new(Mutex::new(Vec::new())),
            propagation_node: Arc::new(Mutex::new(None)),
        }
    }

    /// Derived from the id list rather than counted alongside it: a snapshot
    /// carrying both cannot then report a depth its own ids contradict.
    pub(in crate::lxmf) fn outbound_depth(&self) -> usize {
        self.queued().len()
    }

    pub(in crate::lxmf) fn queued_ids(&self) -> Vec<[u8; 32]> {
        self.queued().clone()
    }

    pub(in crate::lxmf) fn propagation_node(&self) -> Option<[u8; 16]> {
        *self
            .propagation_node
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Restamped from the router every tick, which is what makes this a
    /// following value rather than one latched at the first read.
    ///
    /// Rewritten in place: the writer runs under the core lock on every hook,
    /// and a queue no deeper than the last one costs it no allocation.
    pub(in crate::lxmf) fn publish(
        &self,
        outbound: impl Iterator<Item = [u8; 32]>,
        propagation_node: Option<[u8; 16]>,
    ) {
        let mut queued = self.queued();
        queued.clear();
        queued.extend(outbound);
        drop(queued);
        *self
            .propagation_node
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = propagation_node;
    }

    /// The poisoned lock is recovered from: it is only ever held across a
    /// clear, an extend or a clone, none of which can leave a torn value.
    fn queued(&self) -> MutexGuard<'_, Vec<[u8; 32]>> {
        self.outbound.lock().unwrap_or_else(|e| e.into_inner())
    }
}
