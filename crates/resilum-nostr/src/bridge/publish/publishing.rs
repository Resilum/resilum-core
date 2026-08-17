//! Everything a publication touches outside itself.
//!
//! Borrowed rather than a trait `State` implements: the live `State` carries a
//! mesh sender only a running node can build, so a round settled through
//! `State` itself would need a node behind it.

use std::sync::Arc;

use crate::bridge::deliver;
use crate::bridge::from_mesh::Verdicts;
use crate::bridge::retry::Schedule;
use crate::bridge::state::State;
use crate::config::NostrConfig;
use crate::queue::Queue;
use crate::registry::Registry;

/// How many relays the frame went to, which is how many verdicts an answer
/// to it waits for.
type Broadcast<'a> = Box<dyn Fn(&str) -> usize + 'a>;

/// One acknowledgement, to the mesh address that asked for it.
type Ack<'a> = Box<dyn Fn([u8; 16], &str, &Verdicts) + 'a>;

pub(in crate::bridge) struct Publishing<'a> {
    pub(in crate::bridge) cfg: &'a NostrConfig,
    pub(in crate::bridge) registry: &'a Registry,
    pub(in crate::bridge) queue: &'a Queue,
    pub(in crate::bridge) retry: &'a Schedule,
    pub(in crate::bridge) broadcast: Broadcast<'a>,
    pub(in crate::bridge) ack: Ack<'a>,
}

impl<'a> Publishing<'a> {
    pub(in crate::bridge) fn of(state: &'a Arc<State>) -> Self {
        Self {
            cfg: &state.cfg,
            registry: &state.registry,
            queue: &state.queue,
            retry: &state.retry,
            broadcast: Box::new(|frame| state.broadcast(frame)),
            ack: Box::new(|dest, event_id, verdicts| deliver::ack(state, dest, event_id, verdicts)),
        }
    }
}
