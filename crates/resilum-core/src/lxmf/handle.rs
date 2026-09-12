//! The caller's side of the LXMF stack. The router runs inside the engine's
//! tick, so everything here reaches it over queues.

mod build;
mod outbound;
mod router_state;
mod sink;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use leviculum_lxmf::{DeliveryMethod, Message};
use serde_json::json;

pub(super) use build::channel;
pub(super) use router_state::RouterState;
pub(super) use sink::EventSink;

use crate::error::{Error, Result};

/// Capped because a caller can stop polling while the mesh keeps delivering:
/// the loss is counted and reported as an `overflow` event rather than growing
/// until the process runs out of memory.
pub(super) const MAX_QUEUED_EVENTS: usize = 4096;

pub(super) enum Command {
    /// Built and signed off the core lock, so the hook only has to accept it.
    Send(Box<Message>),
    Announce,
    Stamp(super::stamp::Outcome),
    Requeue {
        message_id: [u8; 32],
        method: DeliveryMethod,
    },
}

pub struct LxmfHandle {
    commands: mpsc::Sender<Command>,
    events: Mutex<mpsc::Receiver<sink::Queued>>,
    inbox: Arc<super::inbox::Inbox>,
    /// Events waiting in `events`, so the sink can refuse a push without
    /// draining the queue to measure it.
    depth: Arc<AtomicUsize>,
    /// What the router reported of itself on the last tick.
    router_state: RouterState,
    registered: Arc<AtomicBool>,
    address_hex: String,
}

impl LxmfHandle {
    /// Derived from the identity alone, so it is valid before the router has
    /// registered.
    #[must_use]
    pub fn address_hex(&self) -> &str {
        &self.address_hex
    }

    /// False for the first tick after start, and permanently once the driver
    /// has detached a panicking processor.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.registered.load(Ordering::Relaxed)
    }

    /// The propagation node a `propagated` send would build against right now,
    /// or `None` when none is selected — the state that gets it refused with
    /// `propagation_node_unavailable` instead. Not latched: a node can be
    /// selected on a tick after this handle was created.
    #[must_use]
    pub fn propagation_node(&self) -> Option<[u8; 16]> {
        self.router_state.propagation_node()
    }

    /// Worth calling on a network change: a peer that has not seen our announce
    /// cannot message us.
    pub fn announce(&self) -> Result<()> {
        self.send(Command::Announce)
    }

    /// Next queued event, as the JSON the FFI hands on. Received messages go
    /// first; a delivery update can wait behind them.
    ///
    /// The poisoned lock is recovered from: it is only ever held across a
    /// `try_recv`, and `None` would claim the mesh went quiet.
    pub fn next_event(&self) -> Option<String> {
        if let Some(json) = self.inbox.pop() {
            return Some(json);
        }
        let refused = self.inbox.take_dropped();
        if refused > 0 {
            return Some(overflow(Overflow::Messages, refused));
        }
        let events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let queued = events.try_recv().ok()?;
            self.depth.fetch_sub(1, Ordering::Relaxed);
            // An announce slot emptied by a concurrent supersede holds nothing
            // new to report, and stopping here would claim the queue ran dry.
            if let Some(json) = queued.into_json() {
                return Some(json);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn queued_events(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    pub(super) fn sender(&self) -> mpsc::Sender<Command> {
        self.commands.clone()
    }

    pub(super) fn router_state(&self) -> RouterState {
        self.router_state.clone()
    }

    fn send(&self, command: Command) -> Result<()> {
        self.commands
            .send(command)
            .map_err(|_| Error::Engine("lxmf processor is gone".into()))
    }
}

/// Which queue overflowed.
pub(super) enum Overflow {
    /// Received messages are gone; only the sender can produce them again.
    Messages,
    /// Entries refused by a full event queue: delivery states superseded
    /// unseen, and announces from peers heard while it was full.
    Delivery,
}

impl Overflow {
    fn token(&self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::Delivery => "delivery",
        }
    }
}

fn overflow(kind: Overflow, dropped: u64) -> String {
    json!({ "type": "overflow", "kind": kind.token(), "dropped": dropped }).to_string()
}
