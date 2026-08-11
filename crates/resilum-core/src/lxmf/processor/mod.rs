//! The LXMF router, driven from inside the engine's tick.
//!
//! It runs here rather than off the public event stream because the events that
//! carry an arriving message — `PacketReceived`, `LinkDataReceived` — are
//! `EventClass::Data` there, which is droppable under load.
//!
//! Both hooks run with the core mutex held, and it is not reentrant. So this
//! type holds no `ReticulumNode` nor anything derived from one — some forty of
//! its methods open by taking that same mutex — and only channels whose sends
//! cannot block. Every side effect is a queue push.

mod absorb;
mod commands;
mod propagation;
mod register;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use leviculum_core::node::NodeEvent;
use leviculum_core::transport::TickOutput;
use leviculum_lxmf::DeliveryStampRequest;
use leviculum_lxmf::router::LxmfRouter;
use leviculum_std::api::Identity;
use leviculum_std::driver::{CoreProcessor, StdNodeCore};
use tokio::sync::mpsc::UnboundedSender;

use super::checkpoint::Checkpoint;
use super::handle::{Command, EventSink};
use crate::config::LxmfConfig;

/// Bounds how long a submitted message waits to be picked up: an event tap
/// only fires when the core has something to say, so the command queue needs a
/// slot of its own.
const POLL_INTERVAL_MS: u64 = 200;

struct Ready {
    router: LxmfRouter,
    delivery_hash: [u8; 16],
    next_announce_ms: u64,
    next_sync_ms: u64,
}

enum State {
    /// Registering needs `&mut StdNodeCore`, which a processor built on the
    /// builder does not have yet.
    Unregistered(Box<Identity>),
    Ready(Box<Ready>),
    Failed,
}

pub(super) struct LxmfProcessor {
    config: LxmfConfig,
    commands: Receiver<Command>,
    events: EventSink,
    stamps: UnboundedSender<DeliveryStampRequest>,
    registered: Arc<AtomicBool>,
    /// `None` when the node has no storage directory, i.e. nowhere durable to
    /// put a queue — in-memory then, as before.
    checkpoint: Option<Checkpoint>,
    state: State,
}

impl LxmfProcessor {
    pub(super) fn new(
        config: LxmfConfig,
        identity: Identity,
        commands: Receiver<Command>,
        events: EventSink,
        stamps: UnboundedSender<DeliveryStampRequest>,
        registered: Arc<AtomicBool>,
        checkpoint: Option<Checkpoint>,
    ) -> Self {
        Self {
            config,
            commands,
            events,
            stamps,
            registered,
            checkpoint,
            state: State::Unregistered(Box::new(identity)),
        }
    }
}

impl CoreProcessor for LxmfProcessor {
    fn on_event(&mut self, core: &mut StdNodeCore, event: &NodeEvent) -> TickOutput {
        let mut out = TickOutput::empty();
        let Some(mut ready) = self.take_ready(core) else {
            return out;
        };
        match ready.router.handle_event(core, event) {
            Ok(output) => self.absorb(&mut ready, core, output, &mut out),
            Err(e) => tracing::warn!(error = ?e, "lxmf handle_event"),
        }
        // Not only on the timer: a live node fires events far more often than
        // `POLL_INTERVAL_MS`, which is what keeps send latency low.
        self.pump_commands(&mut ready, core, &mut out);
        self.state = State::Ready(ready);
        out
    }

    fn on_tick(&mut self, core: &mut StdNodeCore, now_ms: u64) -> TickOutput {
        let mut out = TickOutput::empty();
        let Some(mut ready) = self.take_ready(core) else {
            return out;
        };

        if now_ms >= ready.next_announce_ms {
            ready.next_announce_ms =
                now_ms.saturating_add(self.config.announce_interval.as_millis() as u64);
            self.announce(&mut ready, core, &mut out);
        }
        self.pump_commands(&mut ready, core, &mut out);
        self.drive_propagation(&mut ready, core, now_ms, &mut out);
        match ready.router.tick(core) {
            Ok(output) => self.absorb(&mut ready, core, output, &mut out),
            Err(e) => tracing::warn!(error = ?e, "lxmf tick"),
        }

        self.state = State::Ready(ready);
        // A future instant, never a stale one: a deadline in the past pins the
        // driver to its 1 ms floor.
        let poll = now_ms.saturating_add(POLL_INTERVAL_MS);
        out.next_deadline_ms = Some(match out.next_deadline_ms {
            Some(existing) => existing.min(poll),
            None => poll,
        });
        out
    }
}
