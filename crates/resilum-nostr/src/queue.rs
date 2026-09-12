//! Events not yet delivered, held across restarts.
//!
//! Inbound waits on an offline mesh device; outbound waits on a relay that refused it.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use resilum_store::Writer;

mod entry;
mod store;

pub use entry::{Direction, Entry, Handoff, Queued};

pub struct Queue {
    held: Mutex<store::Held>,
    writer: Writer<store::Change>,
    /// Seconds, converted once: a `Duration` past `i64::MAX` wraps negative on
    /// a cast, and a negative window holds nothing.
    retention: i64,
    per_subscriber: usize,
}

impl Queue {
    pub fn open(path: PathBuf, retention: Duration, per_subscriber: usize) -> Result<Self, String> {
        let held = store::read(&path)?;
        Ok(Self {
            writer: store::spawn_writer(path, held.clone()),
            held: Mutex::new(held),
            retention: seconds(retention),
            per_subscriber,
        })
    }

    /// For a node with no storage directory: held, but not across a restart.
    pub fn ephemeral(retention: Duration, per_subscriber: usize) -> Self {
        Self {
            held: Mutex::new(store::Held::new()),
            writer: store::null_writer(),
            retention: seconds(retention),
            per_subscriber,
        }
    }

    /// Every relay a bridge holds sends the same event and a subscriber pays
    /// airtime for each copy, so the duplicate check is here rather than beside
    /// the call: asking first and pushing second takes the lock twice, and two
    /// upstreams delivering one gift wrap between them both see it free.
    ///
    /// Refuses a subscriber already at the ceiling rather than evicting an older entry: what is
    /// already owed outranks what just arrived. An entry past retention is owed to nobody, so it
    /// neither holds a slot nor answers for a re-offer — `entry.queued_at` stands in for "now".
    pub fn push(&self, entry: Entry) -> Queued {
        let mut held = self.lock();
        let live = held
            .iter()
            .filter(|e| self.within_retention(e.queued_at, entry.queued_at));
        let mut mine = 0;
        for e in live {
            if e.event_id == entry.event_id && e.subscriber == entry.subscriber {
                return Queued::AlreadyHeld;
            }
            if e.subscriber == entry.subscriber {
                mine += 1;
            }
        }
        if mine >= self.per_subscriber {
            return Queued::AtCeiling;
        }
        held.push(entry.clone());
        self.writer.send(store::Change::Push(entry));
        Queued::Held
    }

    /// Unresolved entries still inside the retention window; aging them out is `expire`'s job.
    pub fn due(&self, now: i64) -> Vec<Entry> {
        self.lock()
            .iter()
            .filter(|e| self.within_retention(e.queued_at, now))
            .cloned()
            .collect()
    }

    pub fn owed_to(&self, lxmf: &[u8; 16], now: i64) -> Vec<Entry> {
        self.lock()
            .iter()
            .filter(|e| e.lxmf == *lxmf && self.within_retention(e.queued_at, now))
            .cloned()
            .collect()
    }

    pub fn set_handoff(&self, event_id: &[u8; 32], subscriber: &[u8; 32], handoff: Handoff) {
        let mut held = self.lock();
        let Some(entry) = held
            .iter_mut()
            .find(|e| e.event_id == *event_id && e.subscriber == *subscriber)
        else {
            return;
        };
        entry.handoff = handoff;
        self.writer.send(store::Change::Handoff {
            key: (*event_id, *subscriber),
            handoff,
        });
    }

    pub fn resolve(&self, event_id: &[u8; 32], subscriber: &[u8; 32]) -> Option<Entry> {
        let mut held = self.lock();
        let at = held
            .iter()
            .position(|e| e.event_id == *event_id && e.subscriber == *subscriber)?;
        let entry = held.remove(at);
        self.writer
            .send(store::Change::Remove(vec![(*event_id, *subscriber)]));
        Some(entry)
    }

    pub fn expire(&self, now: i64) -> usize {
        let mut held = self.lock();
        let mut gone = Vec::new();
        held.retain(|e| {
            let keep = self.within_retention(e.queued_at, now);
            if !keep {
                gone.push((e.event_id, e.subscriber));
            }
            keep
        });
        let removed = gone.len();
        if removed > 0 {
            self.writer.send(store::Change::Remove(gone));
        }
        removed
    }

    fn within_retention(&self, queued_at: i64, now: i64) -> bool {
        now - queued_at < self.retention
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, store::Held> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn seconds(retention: Duration) -> i64 {
    i64::try_from(retention.as_secs()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
