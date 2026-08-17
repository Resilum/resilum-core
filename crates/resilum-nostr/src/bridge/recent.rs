//! Event ids admitted into the queue lately, remembered independently of the
//! queue entry itself.
//!
//! `since` is inclusive, so every reconnect re-serves the event the mark
//! stands on. The entry is resolved and dropped once delivered, so without
//! this the subscriber pays for that gift wrap again on every reconnect.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::subscription::Subscription;

/// Only events sharing the newest `created_at` this bridge has marked can be
/// re-served as already-delivered, so this covers a same-second burst far
/// larger than a direct-message feed produces, at two kilobytes a subscriber.
const REMEMBERED: usize = 64;

#[derive(Default)]
pub(super) struct Recent {
    held: Mutex<HashMap<[u8; 32], VecDeque<[u8; 32]>>>,
}

impl Recent {
    #[must_use]
    pub(super) fn seen(&self, subscriber: &[u8; 32], event_id: &[u8; 32]) -> bool {
        self.lock()
            .get(subscriber)
            .is_some_and(|ids| ids.contains(event_id))
    }

    pub(super) fn remember(&self, subscriber: &[u8; 32], event_id: [u8; 32]) {
        let mut held = self.lock();
        let ids = held.entry(*subscriber).or_default();
        ids.push_back(event_id);
        while ids.len() > REMEMBERED {
            ids.pop_front();
        }
    }

    /// A subscriber whose subscription expired is not owed a memory either.
    pub(super) fn keep_only(&self, live: &[Subscription]) {
        self.lock()
            .retain(|subscriber, _| live.iter().any(|sub| sub.pubkey == *subscriber));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 32], VecDeque<[u8; 32]>>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}
