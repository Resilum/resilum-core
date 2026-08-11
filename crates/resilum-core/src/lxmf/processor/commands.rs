//! What the application asked for, applied inside the tick.

use std::sync::mpsc::TryRecvError;

use leviculum_core::DestinationHash;
use leviculum_core::transport::TickOutput;
use leviculum_lxmf::announce;
use leviculum_lxmf::router::RouterOutput;
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready};
use crate::lxmf::handle::Command;
use crate::lxmf::stamp::Outcome;

impl LxmfProcessor {
    /// Drain the command queue. Non-blocking by construction.
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
                        // `submit` returned long ago, so a refusal here can only
                        // reach the app as a delivery update. Without one the
                        // message would sit in the UI as "sending" forever.
                        Err(e) => {
                            tracing::warn!(error = ?e, "lxmf enqueue rejected");
                            self.events.push(crate::lxmf::poll::failed(&message_id));
                        }
                    }
                }
                Ok(Command::Announce) => self.announce(ready, core, out),
                Ok(Command::Stamp(outcome)) => self.apply_stamp(ready, core, outcome, out),
                // Disconnected means the node is shutting down; there is
                // nothing left to drain either way.
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
            }
        }
    }

    fn apply_stamp(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        outcome: Outcome,
        out: &mut TickOutput,
    ) {
        match outcome {
            Outcome::Ready { request, stamp } => {
                match ready
                    .router
                    .set_outbound_stamp_result(core, &request, stamp.to_vec())
                {
                    Ok(output) => self.absorb(ready, core, output, out),
                    Err(e) => tracing::warn!(error = ?e, "lxmf stamp result rejected"),
                }
            }
            // The router has no way to be told a stamp will never arrive, so
            // the message stays queued until it times out on its own. Naming
            // the message is what makes that traceable.
            Outcome::Failed { request, detail } => tracing::warn!(
                message_id = %data_encoding::HEXLOWER.encode(&request.message_id),
                error = %detail,
                "lxmf stamp generation failed",
            ),
        }
    }

    /// Announce the delivery destination, which is what makes this node
    /// addressable at all.
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
