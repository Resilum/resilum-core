//! Store-and-forward, so two peers need not be online at the same moment.

use leviculum_core::transport::TickOutput;
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready};

/// Bounds how stale store-and-forward delivery gets; a reachable peer is
/// delivered to directly and does not wait for this.
const SYNC_INTERVAL_MS: u64 = 300_000;

const ALL_MESSAGES: Option<usize> = None;

impl LxmfProcessor {
    /// Both steps fail until some node announces itself, which is the normal
    /// state on a mesh without one — hence the quiet returns.
    pub(super) fn drive_propagation(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        now_ms: u64,
        out: &mut TickOutput,
    ) {
        if ready.router.outbound_propagation_node().is_none() {
            let preferred = self.config.propagation_node;
            let Ok(output) = ready
                .router
                .select_outbound_propagation_node(core, preferred)
            else {
                return;
            };
            self.absorb(ready, core, output, out);
        }
        if now_ms < ready.next_sync_ms {
            return;
        }
        ready.next_sync_ms = now_ms.saturating_add(SYNC_INTERVAL_MS);
        match ready
            .router
            .request_messages_from_propagation_node(core, ALL_MESSAGES)
        {
            Ok(output) => self.absorb(ready, core, output, out),
            Err(e) => tracing::debug!(error = ?e, "lxmf propagation sync not started"),
        }
    }
}
