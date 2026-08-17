//! The loop where the mesh and the relays meet.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::MissedTickBehavior;

use super::dispatch::{self, Polled};
use super::publish::{Pending, Publishing};
use super::state::State;
use super::{maintain, relay, subscribe};
use crate::upstream::proto::Incoming;

/// The LXMF queue has no readiness signal, so it is asked on a timer. Short
/// enough that a message is not noticeably held up by the bridge itself.
const POLL_EVERY: Duration = Duration::from_millis(100);
const MAINTAIN_EVERY: Duration = Duration::from_secs(60);

/// How many queued events one poll tick may take. The queue has no end while
/// the mesh is busy — announces alone put every LXMF peer in range on it — and
/// an unbounded drain keeps this task inside `drain` for as long as events keep
/// arriving, leaving the relay and maintenance arms unserved.
const MAX_PER_TICK: usize = 256;

pub(super) async fn run(state: Arc<State>, mut relays: UnboundedReceiver<Incoming>) {
    let mut pending = Pending::default();
    let mut poll = tokio::time::interval(POLL_EVERY);
    let mut maintenance = tokio::time::interval(MAINTAIN_EVERY);
    // A tick this loop was too busy to take is not owed afterwards: catching
    // up on ten of them in a row only re-asks a queue that is already empty.
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    maintenance.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut relays_speak = true;
    loop {
        tokio::select! {
            frame = relays.recv(), if relays_speak => match frame {
                Some(frame) => relay::handle(&state, &mut pending, frame),
                // Nothing will arrive from a relay again, but the mesh side
                // still has work: a publish is owed its answer, and a
                // subscription is still written down for a relay to come.
                None => relays_speak = false,
            },
            _ = poll.tick() => {
                drain(&state, &mut pending);
                pending.settle_expired(&Publishing::of(&state));
            }
            _ = maintenance.tick() => maintain::tick(&state, &mut pending),
        }
    }
}

fn drain(state: &Arc<State>, pending: &mut Pending) {
    let now = Instant::now();
    std::iter::from_fn(|| state.lxmf.next_event())
        .take(MAX_PER_TICK)
        .for_each(|json| match dispatch::classify(&json) {
            Polled::Subscribe(data) => subscribe::accept(state, &data),
            Polled::Publish { data, source } => {
                pending.offer(&Publishing::of(state), &data, source, now);
            }
            Polled::Delivered(message_id) => state.delivered(&message_id),
            Polled::NotDelivered(message_id) => state.not_delivered(&message_id),
            Polled::Ignored => {}
        });
}
