//! The minute tick: forget what has aged out, and try again with the rest.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::deliver;
use super::publish::{Pending, Publishing};
use super::state::{self, State};
use super::tie::Tie;
use crate::queue::Direction;

/// How long an LXMF message may go unreported before the bridge stops
/// treating it as still on its way and offers the entry again.
const REPORTED_WITHIN: Duration = Duration::from_secs(600);

pub(super) fn tick(state: &Arc<State>, pending: &mut Pending) {
    let now = state::now();
    let events = state.queue.expire(now);
    let subscribers = state.registry.expire(now);
    if events + subscribers > 0 {
        tracing::info!(events, subscribers, "the bridge let go of what aged out");
    }
    state.forget_stale_ties(REPORTED_WITHIN);
    state.recent.keep_only(&state.registry.live(now));

    let owed = state.queue.due(now);
    let ties: Vec<Tie> = owed.iter().map(Tie::from).collect();
    state.retry.keep_only(&ties);
    let on_the_mesh = state.awaiting_report();
    let bridge = Publishing::of(state);
    let moment = Instant::now();
    for (entry, tie) in owed.into_iter().zip(ties) {
        if !state.retry.due(tie, moment) {
            continue;
        }
        let sent = match entry.direction {
            // A retry of something LXMF is still working on is a duplicate the
            // subscriber pays for in airtime. Only an inbound entry is ever
            // handed to LXMF.
            Direction::Inbound if on_the_mesh.contains(&tie) => continue,
            Direction::Inbound => deliver::inbound(state, entry),
            Direction::Outbound => pending.republish(&bridge, &entry, moment),
        };
        if sent {
            state.retry.attempted(tie, moment);
        }
    }
}
