//! Records which discovery service attached each runtime interface, so status
//! can report the overlay a peer was found through (tor / i2p / yggdrasil /
//! covert). This is a resilum-core concept: leviculum only knows the generic
//! transport (`kind`), never the overlay a peer was discovered on.
//!
//! Interface ids are assigned monotonically, so a freed id is never reused and
//! stale entries are harmless — no detach bookkeeping is needed.

use std::collections::HashMap;
use std::sync::Mutex;

use leviculum_std::InterfaceId;

#[derive(Default)]
pub struct OriginRegistry {
    by_id: Mutex<HashMap<InterfaceId, String>>,
}

impl OriginRegistry {
    /// Record the origin of an interface at attach time.
    pub fn record(&self, id: InterfaceId, origin: impl Into<String>) {
        self.held().insert(id, origin.into());
    }

    /// The origin an interface was discovered on, or `None` when resilum-core
    /// did not attach it (bootstrap, LAN, or a leviculum-managed peer).
    #[must_use]
    pub fn get(&self, id: InterfaceId) -> Option<String> {
        self.held().get(&id).cloned()
    }

    /// Poison is recovered from: the lock only ever guards a single map op, so
    /// there is no torn value to inherit.
    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<InterfaceId, String>> {
        self.by_id.lock().unwrap_or_else(|e| e.into_inner())
    }
}
