//! Frames from a relay: an event to route onto the mesh, or a verdict on
//! one that came off it.

mod admit;

use std::sync::Arc;
use std::time::Instant;

use super::deliver;
use super::publish::{Pending, Publishing, Verdict};
use super::state::{self, State};
use super::tie::Tie;
use crate::event::Event;
use crate::upstream::proto::Incoming;

pub(in crate::bridge) use admit::Stores;

pub(super) fn handle(state: &Arc<State>, pending: &mut Pending, frame: Incoming) {
    match frame {
        Incoming::Event { event } => route(state, *event),
        Incoming::Verdict {
            event_id,
            accepted,
            message,
        } => {
            let verdict = if accepted {
                Verdict::Accepted
            } else {
                Verdict::Rejected(message)
            };
            pending.verdict(&Publishing::of(state), &event_id, verdict);
        }
        Incoming::EndOfStored { subscription } => {
            tracing::debug!(subscription, "a relay finished sending what it had stored");
        }
        // The connection stays up and `is_up` stays true, so nothing else
        // here would ever say that this subscriber is being served nothing.
        Incoming::Closed {
            subscription,
            message,
        } => tracing::warn!(subscription, message, "a relay refused a subscription"),
        Incoming::Notice(message) => tracing::info!(message, "a relay said something"),
    }
}

/// One event may name several subscribers, and each is owed its own copy.
fn route(state: &Arc<State>, event: Event) {
    for entry in admit::admit_all(&state.stores(), &event, state::now()) {
        // Before the send, which records its tie only once the message is
        // packed on another thread: until then this is the only thing
        // keeping the next maintenance tick from offering the entry again.
        state.retry.attempted(Tie::from(&entry), Instant::now());
        deliver::inbound(state, entry);
    }
}
