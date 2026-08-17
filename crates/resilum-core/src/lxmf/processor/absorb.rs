//! Draining one router output.

use leviculum_core::transport::TickOutput;
use leviculum_lxmf::router::{RouterEvent, RouterOutput};
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready};

/// A core call inside a hook returns its events synchronously and the driver
/// never hands them back, so re-feeding them is the consumer's job. Bounded
/// rather than run to a fixpoint: an LXMF event provoking another is
/// legitimate, and an unbounded loop under the core lock hangs the node.
const MAX_ABSORB_ROUNDS: usize = 8;

impl LxmfProcessor {
    pub(super) fn absorb(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        first: RouterOutput,
        out: &mut TickOutput,
    ) {
        let mut queue = std::collections::VecDeque::from([first]);
        let mut rounds = 0usize;
        let mut checkpoint_due = false;
        // One reading for the whole operation: every event below came out of
        // the same core call, so they were all heard at the same moment.
        let heard_at = crate::wall_clock::unix_now();
        while let Some(router_output) = queue.pop_front() {
            for event in router_output.events {
                checkpoint_due |= matches!(event, RouterEvent::PersistenceRequested);
                if let Some(event) = self.order_stamp(event) {
                    self.enqueue_for_caller(event, heard_at);
                }
            }
            let mut core_output = router_output.core;
            let events = std::mem::take(&mut core_output.events);
            out.merge(core_output);

            rounds += 1;
            let refeed = rounds <= MAX_ABSORB_ROUNDS;
            if !refeed && !events.is_empty() {
                tracing::warn!(
                    events = events.len(),
                    "lxmf absorb bound reached; events forwarded without re-entering the router",
                );
            }
            for event in events {
                if refeed {
                    match ready.router.handle_event(core, &event) {
                        Ok(next) => queue.push_back(next),
                        Err(e) => tracing::warn!(error = ?e, "lxmf handle_event"),
                    }
                }
                out.events.push(event);
            }
        }
        if checkpoint_due {
            self.checkpoint(ready);
        }
    }

    /// Take the router's checkpoint once the whole operation has settled.
    ///
    /// `persist` looks like disk I/O and is not: the storage hands the bytes to
    /// a writer thread ([`crate::lxmf::checkpoint`]). What it does cost here is
    /// serialising the snapshot, which is why it runs once per operation rather
    /// than once per event — the router asks on every queue change.
    fn checkpoint(&mut self, ready: &mut Ready) {
        let Some(checkpoint) = self.checkpoint.as_mut() else {
            return;
        };
        if let Err(e) = ready.router.persist(checkpoint) {
            tracing::warn!(error = ?e, "lxmf checkpoint not written");
        }
    }

    /// A stamp request goes to the mining thread and no further. Both answers
    /// come back as `Command::Stamp`; with nobody grinding, the message waits
    /// for a stamp that never arrives, and a propagated one waits without ever
    /// spending a delivery attempt — so it reaches no verdict at all.
    fn order_stamp(&mut self, event: RouterEvent) -> Option<RouterEvent> {
        match event {
            RouterEvent::StampPending(request) => {
                self.stamps.delivery(request);
                None
            }
            RouterEvent::PropagationStampPending(request) => {
                self.stamps.propagation(request);
                None
            }
            other => Some(other),
        }
    }

    fn enqueue_for_caller(&mut self, event: RouterEvent, heard_at: f64) {
        let Some(json) = crate::lxmf::poll::event_to_json(&event, heard_at) else {
            tracing::debug!(event = ?event, "lxmf router event");
            return;
        };
        match event {
            // A delivery update stays in the queue because the next one
            // supersedes it; a message has no such successor.
            RouterEvent::MessageReceived(_) => self.inbox.push(json),
            RouterEvent::PeerAnnounced { destination, .. } => {
                self.events.push_announce(*destination.as_bytes(), json);
            }
            _ => self.events.push(json),
        }
    }
}
