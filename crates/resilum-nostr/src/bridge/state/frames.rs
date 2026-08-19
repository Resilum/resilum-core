//! Cutting the live subscribers into the `REQ` frames that stand for them.

use std::collections::BTreeMap;

use data_encoding::HEXLOWER;

use crate::event::{GIFT_WRAP_KIND, NIP59_BACKDATE};
use crate::registry::{BatchId, LiveMark};
use crate::upstream::proto::{self, Filter};

/// A subscriber and the point it resumes from.
pub(super) type Mark = ([u8; 32], i64);

/// One `REQ` per batch actually populated. Membership is decided at
/// registry-accept time and stays fixed for the life of a subscription, so
/// this only has to group by the batch each mark already carries — it never
/// reshuffles anyone, which is what keeps every other batch's frame
/// unchanged when one subscriber joins, refreshes or expires.
pub(super) fn requests(kinds: &[u32], marks: &[LiveMark]) -> Vec<String> {
    let mut batches: BTreeMap<BatchId, Vec<Mark>> = BTreeMap::new();
    for mark in marks {
        batches
            .entry(mark.batch)
            .or_default()
            .push((mark.pubkey, mark.last_seen));
    }
    batches
        .into_iter()
        .map(|(batch, chunk)| batch_frame(kinds, batch, &chunk))
        .collect()
}

pub(super) fn batch_frame(kinds: &[u32], batch: BatchId, marks: &[Mark]) -> String {
    proto::request_frame(&id(batch), &filters(kinds, marks))
}

/// The id names the batch and nothing else. Reissuing under the same id is
/// what replaces a subscription in place rather than opening a second one.
fn id(batch: BatchId) -> String {
    format!("b{batch}")
}

fn filters(kinds: &[u32], chunk: &[Mark]) -> Vec<Filter> {
    chunk
        .iter()
        .map(|(pubkey, last_seen)| Filter {
            kinds: kinds.to_vec(),
            subscriber_pubkey_hex: HEXLOWER.encode(pubkey),
            since: Some(resume_from(kinds, *last_seen)),
        })
        .collect()
}

fn resume_from(kinds: &[u32], last_seen: i64) -> i64 {
    if kinds.contains(&GIFT_WRAP_KIND) {
        return last_seen.saturating_sub(NIP59_BACKDATE).max(0);
    }
    last_seen
}

#[cfg(test)]
mod tests;
