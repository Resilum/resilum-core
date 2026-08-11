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
        while let Some(router_output) = queue.pop_front() {
            for event in router_output.events {
                checkpoint_due |= matches!(event, RouterEvent::PersistenceRequested);
                self.report(event);
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

    /// Turn one router event into what the application sees.
    fn report(&mut self, event: RouterEvent) {
        if let RouterEvent::StampPending(request) = event {
            // Off the lock: mining is unbounded work. The answer comes back as
            // `Command::Stamp`; with no miner the message waits for one that
            // never arrives.
            if self.stamps.send(request).is_err() {
                tracing::error!("lxmf stamp miner is gone; priced messages cannot be sent");
            }
            return;
        }
        match crate::lxmf::poll::event_to_json(&event) {
            Some(json) => self.events.push(json),
            None => tracing::debug!(event = ?event, "lxmf router event"),
        }
    }
}
