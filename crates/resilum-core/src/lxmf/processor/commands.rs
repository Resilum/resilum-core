//! What the application asked for, applied inside the tick.

use std::sync::mpsc::TryRecvError;

use data_encoding::HEXLOWER;
use leviculum_core::DestinationHash;
use leviculum_core::transport::TickOutput;
use leviculum_lxmf::announce;
use leviculum_lxmf::router::{RouterError, RouterOutput};
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready};
use crate::lxmf::handle::{Command, EventSink};

mod requeue;
mod stamp;

impl LxmfProcessor {
    /// Non-blocking by construction: this runs with the core mutex held.
    pub(super) fn pump_commands(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        out: &mut TickOutput,
    ) {
        loop {
            match self.commands.try_recv() {
                Ok(Command::Send(message)) => {
                    let message_id = message.message_id;
                    match ready.router.enqueue(core, *message) {
                        Ok(output) => self.absorb(ready, core, output, out),
                        Err(e) => report_enqueue_error(&mut self.events, &message_id, e),
                    }
                }
                Ok(Command::Announce) => self.announce(ready, core, out),
                Ok(Command::Stamp(outcome)) => self.apply_stamp(ready, core, outcome, out),
                Ok(Command::Requeue { message_id, method }) => {
                    self.requeue(ready, core, message_id, method, out);
                }
                // Disconnected means the node is shutting down; there is
                // nothing left to drain either way.
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
            }
        }
    }

    /// What makes this node addressable at all.
    pub(super) fn announce(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        out: &mut TickOutput,
    ) {
        // A `None` stamp cost asks no proof of work of anyone messaging us.
        let name = self.config.display_name.clone().unwrap_or_default();
        let app_data = announce::delivery(Some(name.as_bytes()), None);
        let hash = DestinationHash::new(ready.delivery_hash);
        match core.announce_destination(&hash, Some(&app_data)) {
            Ok(core_output) => {
                tracing::info!(
                    address = %HEXLOWER.encode(&ready.delivery_hash),
                    "lxmf announce sent",
                );
                // Through `absorb` rather than merged straight into `out`: the
                // call ends in event processing, so its output can carry events
                // the router still needs to see.
                let output = RouterOutput {
                    core: core_output,
                    events: Vec::new(),
                };
                self.absorb(ready, core, output, out);
            }
            Err(e) => tracing::warn!(error = ?e, "lxmf announce failed"),
        }
    }
}

/// Turn an enqueue refusal into what the caller is told, if anything.
///
/// `submit` returned `Ok` long before this runs — it only enqueued a channel
/// command — so a refusal here is the only chance to reach the caller, and for
/// most errors the answer is a synthetic `failed` event. Without one the
/// message stays in the caller's model as "sending" forever.
///
/// `Duplicate` is the one exception: the router already holds this id in
/// `outbound` and is retrying it on its own, so the verdict will come from
/// that in-flight attempt. A second one from here would contradict it.
fn report_enqueue_error(events: &mut EventSink, message_id: &[u8; 32], error: RouterError) {
    if error == RouterError::Duplicate {
        tracing::debug!(
            message_id = %HEXLOWER.encode(message_id),
            "lxmf enqueue duplicate, already in flight",
        );
        return;
    }
    tracing::warn!(error = ?error, "lxmf enqueue rejected");
    events.push(crate::lxmf::poll::failed(message_id, &error));
}

#[cfg(test)]
mod tests;
