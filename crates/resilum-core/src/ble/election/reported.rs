use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::Facts;

#[derive(Clone, Default)]
pub struct WhatThePlatformKnows {
    facts: Arc<Mutex<Facts>>,
    can_host_at_all: Arc<AtomicBool>,
}

impl WhatThePlatformKnows {
    #[must_use]
    pub fn nothing_yet() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn unless_a_platform_says(can_host_at_all: bool) -> Self {
        let known = Self::default();
        known
            .can_host_at_all
            .store(can_host_at_all, Ordering::Relaxed);
        known
    }

    pub fn report(&self, facts: Facts, can_host_at_all: bool) {
        *self.facts.lock().unwrap_or_else(|e| e.into_inner()) = facts;
        self.can_host_at_all
            .store(can_host_at_all, Ordering::Relaxed);
    }

    #[must_use]
    pub fn can_host_at_all(&self) -> bool {
        self.can_host_at_all.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn read(&self, neighbours_heard: u8) -> Facts {
        let told = *self.facts.lock().unwrap_or_else(|e| e.into_inner());
        Facts {
            neighbours_heard,
            ..super::what_this_host_can_answer(told)
        }
    }
}

#[cfg(test)]
mod tests;
