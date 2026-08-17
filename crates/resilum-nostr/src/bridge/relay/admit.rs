//! Whether an event a relay sent becomes something owed to a subscriber.

mod filter;

use crate::config::NostrConfig;
use crate::event::Event;
use crate::queue::{Direction, Entry, Queued};
use crate::registry::Registry;

use super::super::recent::Recent;

pub(in crate::bridge) struct Stores<'a> {
    pub(in crate::bridge) cfg: &'a NostrConfig,
    pub(in crate::bridge) registry: &'a Registry,
    pub(in crate::bridge) queue: &'a crate::queue::Queue,
    pub(in crate::bridge) recent: &'a Recent,
}

/// Who an arriving event is owed to. A batched `REQ` carries one filter per
/// subscriber and returns anything matching any of them, so the subscription
/// id names the batch rather than a destination — the `p` tags are what the
/// filters were written on, and so what routing reads.
///
/// Empty when the event names nobody this bridge holds, which is a refusal:
/// there is no subscriber to fall back to.
pub(super) fn admit_all(stores: &Stores<'_>, event: &Event, now: i64) -> Vec<Entry> {
    filter::addressed(event)
        .into_iter()
        .filter_map(|subscriber| admit(stores, subscriber, event, now))
        .collect()
}

pub(super) fn admit(
    stores: &Stores<'_>,
    subscriber: [u8; 32],
    event: &Event,
    now: i64,
) -> Option<Entry> {
    let Some(lxmf) = stores.registry.lxmf_for(&subscriber) else {
        tracing::debug!("a relay sent an event for a subscriber this bridge does not hold");
        return None;
    };
    let event_json = serde_json::to_string(event).ok()?;
    if let Err(reason) = filter::asked_for(stores.cfg, &subscriber, event, &event_json) {
        tracing::debug!(reason, "a relay sent an event nobody asked it for");
        return None;
    }
    // Before the id is trusted for anything: a kind-1059 gift wrap is
    // unauthenticated by design, so an unsigned one claiming a real event's
    // id would otherwise suppress the real one when it arrives.
    if let Err(reason) = event.verify() {
        tracing::warn!(%reason, "a relay sent an event that does not verify");
        return None;
    }
    let event_id = event.id_bytes()?;
    if stores.recent.seen(&subscriber, &event_id) {
        return None;
    }
    let entry = Entry {
        direction: Direction::Inbound,
        subscriber,
        lxmf,
        event_id,
        event_json: event_json.into(),
        queued_at: now,
    };
    match stores.queue.push(entry.clone()) {
        Queued::Held => {}
        // The mark stays where it is: an event dropped at the ceiling is one
        // a later `REQ` should still ask for.
        Queued::AtCeiling => {
            tracing::warn!("a subscriber is at its queue ceiling; an event was dropped");
            return None;
        }
        Queued::AlreadyHeld => return None,
    }
    stores.recent.remember(&subscriber, event_id);
    mark(stores.registry, &subscriber, event.created_at, now);
    Some(entry)
}

/// The mark is where every later `REQ` resumes from, and an event's
/// `created_at` is whatever its author wrote. One event dated in the next
/// century would otherwise silence this subscriber for good, across
/// restarts, so the mark never runs ahead of our own clock — and never goes
/// backwards, which would re-fetch what has already been delivered.
fn mark(registry: &Registry, subscriber: &[u8; 32], created_at: i64, now: i64) {
    let mark = created_at.min(now);
    if registry
        .last_seen(subscriber)
        .is_some_and(|seen| mark <= seen)
    {
        return;
    }
    registry.mark_seen(subscriber, mark);
}

#[cfg(test)]
mod tests;
