//! What the stamp thread came back with, applied to the message that waits on
//! it.

use leviculum_core::transport::TickOutput;
use leviculum_lxmf::router::RouterError;
use leviculum_lxmf::{DeliveryStampRequest, PropagationStampRequest};
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready};
use crate::lxmf::stamp::Outcome;

impl LxmfProcessor {
    pub(super) fn apply_stamp(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        outcome: Outcome,
        out: &mut TickOutput,
    ) {
        match outcome {
            Outcome::Delivery { request, stamp } => {
                self.apply_delivery(ready, core, request, stamp, out);
            }
            Outcome::Propagation { request, stamp } => {
                self.apply_propagation(ready, core, request, stamp, out);
            }
        }
    }

    /// The recipient's stamp, which prices being messaged at all.
    fn apply_delivery(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        request: DeliveryStampRequest,
        stamp: Result<[u8; 32], String>,
        out: &mut TickOutput,
    ) {
        let stamp = match stamp {
            Ok(stamp) => stamp,
            // The router has no way to be told a stamp will never arrive, so
            // the message stays queued until it times out on its own. Naming
            // the message is what makes that traceable.
            Err(detail) => {
                tracing::warn!(
                    message_id = %data_encoding::HEXLOWER.encode(&request.message_id),
                    error = %detail,
                    "lxmf stamp generation failed",
                );
                return;
            }
        };
        match ready
            .router
            .set_outbound_stamp_result(core, &request, stamp.to_vec())
        {
            Ok(output) => self.absorb(ready, core, output, out),
            Err(e) => tracing::warn!(error = ?e, "lxmf stamp result rejected"),
        }
    }

    /// The propagation node's stamp, which prices depositing a message for a
    /// peer that is not online.
    fn apply_propagation(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        request: PropagationStampRequest,
        stamp: Result<[u8; 32], String>,
        out: &mut TickOutput,
    ) {
        // Whatever came back, this message is off the stamp thread: the router asks
        // again on its next pass if it still wants a stamp.
        self.stamps.finished(&request);
        let stamp = match stamp {
            Ok(stamp) => stamp,
            Err(detail) => {
                tracing::warn!(
                    message_id = %data_encoding::HEXLOWER.encode(&request.message_id),
                    error = %detail,
                    "lxmf propagation stamp generation failed",
                );
                return;
            }
        };
        match ready
            .router
            .set_outbound_propagation_stamp_result(&request, stamp, core.now_ms())
        {
            Ok(output) => self.absorb(ready, core, output, out),
            // Expected, not a fault: the entry moved on while the stamp was
            // being mined.
            Err(RouterError::StaleStampRequest) => tracing::debug!(
                message_id = %data_encoding::HEXLOWER.encode(&request.message_id),
                "lxmf propagation stamp no longer wanted",
            ),
            Err(e) => tracing::warn!(error = ?e, "lxmf propagation stamp result rejected"),
        }
    }
}
