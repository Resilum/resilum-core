//! Reading the registry: everything that answers a question without changing
//! what is held, and so without ever reaching the writer.

use super::{BatchId, Registry};
use crate::subscription::Subscription;

/// One live subscriber, as a `REQ` filter needs it.
pub(crate) struct LiveMark {
    pub(crate) batch: BatchId,
    pub(crate) pubkey: [u8; 32],
    pub(crate) last_seen: i64,
}

impl Registry {
    /// The batch a held subscriber's `REQ` filter rides in, unrelated to
    /// whether the subscriber is still live.
    #[cfg(test)]
    pub(crate) fn batch_of(&self, pubkey: &[u8; 32]) -> Option<BatchId> {
        self.lock().get(pubkey).map(|record| record.batch)
    }

    pub(crate) fn lxmf_for(&self, pubkey: &[u8; 32]) -> Option<[u8; 16]> {
        self.lock().get(pubkey).map(|record| record.lxmf)
    }

    pub(crate) fn last_seen(&self, pubkey: &[u8; 32]) -> Option<i64> {
        self.lock().get(pubkey).map(|record| record.last_seen)
    }

    /// Every subscriber registered at an LXMF address — the reverse of
    /// `lxmf_for`, and a set rather than one pubkey because the mapping is
    /// not one to one: one device may register several keys, and nothing in
    /// the wire contract stops it.
    pub(crate) fn pubkeys_at(&self, lxmf: &[u8; 16]) -> Vec<[u8; 32]> {
        self.lock()
            .iter()
            .filter(|(_, record)| record.lxmf == *lxmf)
            .map(|(pubkey, _)| *pubkey)
            .collect()
    }

    /// Does not remove anything; `expire` is what drops the stored map.
    pub(crate) fn live(&self, now: i64) -> Vec<Subscription> {
        self.lock()
            .iter()
            .filter(|(_, record)| now - record.created_at < self.retention)
            .map(|(pubkey, record)| Subscription {
                pubkey: *pubkey,
                lxmf: record.lxmf,
                created_at: record.created_at,
            })
            .collect()
    }

    /// Everything a `REQ` frame is built from, in one lock rather than one
    /// acquisition per subscriber.
    pub(crate) fn live_marks(&self, now: i64) -> Vec<LiveMark> {
        self.lock()
            .iter()
            .filter(|(_, record)| now - record.created_at < self.retention)
            .map(|(pubkey, record)| LiveMark {
                batch: record.batch,
                pubkey: *pubkey,
                last_seen: record.last_seen,
            })
            .collect()
    }
}
