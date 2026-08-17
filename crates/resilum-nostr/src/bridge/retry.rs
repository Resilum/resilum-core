//! When an unresolved entry may be sent again.
//!
//! `queue.due` returns everything still owed, and retention is days: without
//! a schedule a device that is off for a week would have every entry
//! re-submitted on every minute tick, which on a bandwidth-scarce link is
//! the bridge shouting at a peer that cannot hear it.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::tie::Tie;

/// The first wait is one tick; doubling from there caps at an hour, which
/// over a seven-day retention is under two hundred attempts rather than ten
/// thousand. Same shape as the upstream reconnect.
const FIRST: Duration = Duration::from_secs(60);
const CAP: Duration = Duration::from_secs(3600);

#[derive(Default)]
pub(super) struct Schedule {
    held: Mutex<HashMap<Tie, Attempt>>,
}

struct Attempt {
    count: u32,
    next: Instant,
}

impl Schedule {
    #[must_use]
    pub(super) fn due(&self, tie: Tie, now: Instant) -> bool {
        self.lock()
            .get(&tie)
            .is_none_or(|attempt| attempt.next <= now)
    }

    pub(super) fn attempted(&self, tie: Tie, now: Instant) {
        let mut held = self.lock();
        let attempt = held.entry(tie).or_insert(Attempt {
            count: 0,
            next: now,
        });
        attempt.count = attempt.count.saturating_add(1);
        attempt.next = now + backoff(attempt.count);
    }

    pub(super) fn keep_only(&self, owed: &[Tie]) {
        let live: HashSet<Tie> = owed.iter().copied().collect();
        self.lock().retain(|tie, _| live.contains(tie));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Tie, Attempt>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn backoff(count: u32) -> Duration {
    let doublings = count.saturating_sub(1).min(6);
    (FIRST * (1 << doublings)).min(CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tie() -> Tie {
        Tie {
            event_id: [1u8; 32],
            subscriber: [7u8; 32],
        }
    }

    #[test]
    fn every_attempt_waits_longer_than_the_one_before() {
        let schedule = Schedule::default();
        let start = Instant::now();

        assert!(schedule.due(tie(), start));
        schedule.attempted(tie(), start);
        assert!(!schedule.due(tie(), start + Duration::from_secs(59)));

        let second = start + Duration::from_secs(61);
        assert!(schedule.due(tie(), second));
        schedule.attempted(tie(), second);
        assert!(!schedule.due(tie(), second + Duration::from_secs(119)));
        assert!(schedule.due(tie(), second + Duration::from_secs(121)));
    }

    #[test]
    fn the_wait_stops_growing_at_an_hour() {
        assert_eq!(backoff(20), CAP);
    }

    #[test]
    fn an_entry_that_left_the_queue_is_forgotten() {
        let schedule = Schedule::default();
        let start = Instant::now();
        schedule.attempted(tie(), start);

        schedule.keep_only(&[]);

        assert!(schedule.due(tie(), start));
    }
}
