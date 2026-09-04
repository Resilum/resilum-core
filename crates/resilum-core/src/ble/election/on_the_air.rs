use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const NOT_SINCE_WE_LOOKED: u64 = u64::MAX;

pub const STOPS_COUNTING_AFTER_MS: u64 = 30_000;

#[derive(Clone)]
pub struct SomeoneElsesGroup(Arc<AtomicU64>);

impl Default for SomeoneElsesGroup {
    fn default() -> Self {
        Self(Arc::new(AtomicU64::new(NOT_SINCE_WE_LOOKED)))
    }
}

impl SomeoneElsesGroup {
    #[must_use]
    pub fn none_heard_yet() -> Self {
        Self::default()
    }

    pub fn heard_at(&self, now_ms: u64) {
        self.0.store(now_ms, Ordering::Relaxed);
    }

    pub fn forget_it_if_it_has_gone_quiet(&self, now_ms: u64) {
        let heard = self.0.load(Ordering::Relaxed);
        if heard != NOT_SINCE_WE_LOOKED && now_ms.saturating_sub(heard) >= STOPS_COUNTING_AFTER_MS {
            self.forget_it();
        }
    }

    pub fn forget_it(&self) {
        self.0.store(NOT_SINCE_WE_LOOKED, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_up(&self) -> bool {
        self.0.load(Ordering::Relaxed) != NOT_SINCE_WE_LOOKED
    }
}

#[cfg(test)]
mod tests;
