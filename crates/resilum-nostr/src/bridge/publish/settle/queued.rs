//! What a settled event becomes on the queue side: resolved for its
//! subscriber, or held for the next attempt when nothing took it.

use std::sync::Arc;
use std::time::Instant;

use crate::bridge::publish::{Publication, Publishing};
use crate::bridge::state;
use crate::queue::{Direction, Entry, Handoff, Queued};

/// Runs once for the event: the queue is keyed by it and its subscriber, and
/// every peer that forwarded it named the same pair.
pub(super) fn record(bridge: &Publishing<'_>, round: &Publication) {
    if round.accepted > 0 {
        bridge
            .queue
            .resolve(&round.tie.event_id, &round.tie.subscriber);
        return;
    }
    // The address it first arrived at, whose sender has been waiting longest
    // and whose retry is therefore the one owed. Nothing on the list means
    // nothing is owed: a queued event offered again is already held.
    let Some(&lxmf) = round.reply_to.first() else {
        return;
    };
    let entry = Entry {
        direction: Direction::Outbound,
        subscriber: round.tie.subscriber,
        lxmf,
        event_id: round.tie.event_id,
        event_json: Arc::clone(&round.event_json),
        queued_at: state::now(),
        handoff: Handoff::default(),
    };
    match bridge.queue.push(entry) {
        // It has just been offered, so the next tick is not its next attempt.
        Queued::Held => bridge.retry.attempted(round.tie, Instant::now()),
        Queued::AlreadyHeld => {}
        Queued::AtCeiling => {
            tracing::warn!("an event no relay took could not be held for retry");
        }
    }
}
