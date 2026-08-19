//! Event ids already admitted, so a re-served one costs no mesh airtime.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::event::NIP59_BACKDATE;
use crate::subscription::Subscription;

const FLOOD_CEILING: usize = 1024;

struct Seen {
    event_id: [u8; 32],
    created_at: i64,
}

#[derive(Default)]
struct Memory {
    seen: VecDeque<Seen>,
    newest: i64,
}

impl Memory {
    fn holds(&self, event_id: &[u8; 32]) -> bool {
        self.seen.iter().any(|seen| seen.event_id == *event_id)
    }

    fn remember(&mut self, event_id: [u8; 32], created_at: i64) {
        self.seen.push_back(Seen {
            event_id,
            created_at,
        });
        self.newest = self.newest.max(created_at);
        let resumes_from = self.newest.saturating_sub(NIP59_BACKDATE);
        self.seen.retain(|seen| seen.created_at >= resumes_from);
        while self.seen.len() > FLOOD_CEILING {
            self.seen.pop_front();
        }
    }
}

#[derive(Default)]
pub(super) struct Recent {
    held: Mutex<HashMap<[u8; 32], Memory>>,
}

impl Recent {
    #[must_use]
    pub(super) fn seen(&self, subscriber: &[u8; 32], event_id: &[u8; 32]) -> bool {
        self.lock()
            .get(subscriber)
            .is_some_and(|memory| memory.holds(event_id))
    }

    pub(super) fn remember(&self, subscriber: &[u8; 32], event_id: [u8; 32], created_at: i64) {
        self.lock()
            .entry(*subscriber)
            .or_default()
            .remember(event_id, created_at);
    }

    /// A subscriber whose subscription expired is not owed a memory either.
    pub(super) fn keep_only(&self, live: &[Subscription]) {
        self.lock()
            .retain(|subscriber, _| live.iter().any(|sub| sub.pubkey == *subscriber));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 32], Memory>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests;
