//! Accepted subscriptions, held across restarts.
//!
//! This is the source of truth for what the bridge asks upstream and where
//! it delivers: a pubkey we have not seen has nowhere to route events to.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use crate::subscription::Subscription;
use crate::writer::Writer;

mod admit;
mod batch;
mod query;
mod store;
#[cfg(test)]
mod tests;

pub(crate) use admit::AcceptError;
#[cfg(test)]
pub(crate) use admit::FILTERS_PER_REQUEST;
pub(crate) use batch::BatchId;
pub(crate) use query::LiveMark;

pub(crate) struct Registry {
    held: Mutex<store::Held>,
    writer: Writer<store::Change>,
    retention: i64,
}

impl Registry {
    pub(crate) fn open(path: PathBuf, retention: Duration) -> Result<Self, String> {
        let held = store::read(&path)?;
        Ok(Self {
            writer: store::spawn_writer(path, held.clone()),
            held: Mutex::new(held),
            retention: seconds(retention),
        })
    }

    /// For a node with no storage directory: held, but not across a restart.
    pub(crate) fn ephemeral(retention: Duration) -> Self {
        Self {
            held: Mutex::new(store::Held::new()),
            writer: store::null_writer(),
            retention: seconds(retention),
        }
    }

    /// `Ok(Some(batch))` only when the subscriber is new to that batch — a
    /// refresh reuses the one it already holds, and reissuing a `REQ` no
    /// subscriber was added to or dropped from would tell the relay nothing
    /// it does not already know.
    pub(crate) fn ensure_fresh(&self, sub: &Subscription, now: i64) -> Result<(), AcceptError> {
        let held = self.lock();
        admit::ensure_within_retention(sub, now, self.retention)?;
        admit::ensure_not_replayed(&held, sub)
    }

    pub(crate) fn accept(
        &self,
        sub: Subscription,
        now: i64,
    ) -> Result<Option<BatchId>, AcceptError> {
        let mut held = self.lock();
        admit::ensure_within_retention(&sub, now, self.retention)?;
        admit::ensure_not_replayed(&held, &sub)?;
        admit::ensure_room_for(&held, &sub, now, self.retention)?;
        let placement = admit::place(&held, &sub, now, self.retention);
        let record = store::Record {
            lxmf: sub.lxmf,
            created_at: sub.created_at,
            last_seen: admit::carried_mark(&held, &sub, now),
            batch: placement.batch(),
        };
        held.insert(sub.pubkey, record);
        self.writer.send(store::Change::Upsert(sub.pubkey, record));
        Ok(placement.joined())
    }

    /// Drops what `live` already hides, so a subscriber nobody refreshed stops
    /// costing a line on disk.
    pub(crate) fn expire(&self, now: i64) -> usize {
        let mut held = self.lock();
        let gone: Vec<[u8; 32]> = held
            .iter()
            .filter(|(_, record)| now - record.created_at >= self.retention)
            .map(|(pubkey, _)| *pubkey)
            .collect();
        if gone.is_empty() {
            return 0;
        }
        held.retain(|pubkey, _| !gone.contains(pubkey));
        let removed = gone.len();
        self.writer.send(store::Change::Remove(gone));
        removed
    }

    /// The mark only ever moves forward: an event arriving out of order is
    /// dated before events already delivered, and resuming from its date
    /// would re-fetch and re-deliver everything in between.
    pub(crate) fn mark_seen(&self, pubkey: &[u8; 32], created_at: i64) {
        let mut held = self.lock();
        if let Some(record) = held.get_mut(pubkey).filter(|r| created_at > r.last_seen) {
            record.last_seen = created_at;
            self.writer.send(store::Change::Upsert(*pubkey, *record));
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, store::Held> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Saturating: a window past `i64::MAX` cast blindly comes out negative and
/// reads as "everything has already lapsed".
fn seconds(retention: Duration) -> i64 {
    i64::try_from(retention.as_secs()).unwrap_or(i64::MAX)
}
