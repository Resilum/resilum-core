//! Moving a queued message onto another delivery method.

use leviculum_core::transport::TickOutput;
use leviculum_lxmf::DeliveryMethod;
use leviculum_lxmf::router::{MessageState, RouterError, RouterEvent, RouterOutput};
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready, report_enqueue_error};
use crate::lxmf::poll::requeue_refused;

impl LxmfProcessor {
    /// The queued message is re-queued verbatim rather than rebuilt from its
    /// parts. Nothing has to be re-signed, and an id that came back different
    /// would be a defect rather than a possibility to handle.
    pub(super) fn requeue(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        message_id: [u8; 32],
        method: DeliveryMethod,
        out: &mut TickOutput,
    ) {
        let queued = ready
            .router
            .outbound()
            .get(&message_id)
            .map(|entry| entry.message().clone());
        let Some(original) = queued else {
            self.events
                .push(requeue_refused(&message_id, &RouterError::NotFound));
            return;
        };
        // `enqueue` takes the message by value and hands nothing back when it
        // refuses one, so the copy that outlives a refusal has to exist before
        // the call rather than be made on the path that needs it.
        let mut retried = original.clone();
        retried.method = method;

        match ready.router.cancel(core, &message_id) {
            Ok(output) => {
                let output = without_cancelled(output, &message_id);
                self.absorb(ready, core, output, out);
            }
            Err(e) => {
                self.events.push(requeue_refused(&message_id, &e));
                return;
            }
        }

        let refusal = match ready.router.enqueue(core, retried) {
            Ok(output) => {
                self.absorb(ready, core, output, out);
                return;
            }
            Err(e) => e,
        };
        self.restore(ready, core, original, out);
        self.events.push(requeue_refused(&message_id, &refusal));
    }

    /// Put back the message the cancel above already removed, after the new
    /// method turned out to be one the router will not take — escalating to
    /// `propagated` with no propagation node selected, most likely. Losing a
    /// queued message over a refused re-queue would be the worse outcome by far.
    fn restore(
        &mut self,
        ready: &mut Ready,
        core: &mut StdNodeCore,
        original: leviculum_lxmf::Message,
        out: &mut TickOutput,
    ) {
        let message_id = original.message_id;
        match ready.router.enqueue(core, original) {
            Ok(output) => self.absorb(ready, core, output, out),
            Err(e) => report_enqueue_error(&mut self.events, &message_id, e),
        }
    }
}

/// Drop the `Cancelled` this re-queue's own cancel emitted, and nothing else:
/// the message is being resent under a new method as the event is read, so a
/// terminal state for it would be the same lie as reporting a resubmitted
/// in-flight message as `failed`.
///
/// Scoped by the output it filters, which is the one `cancel` just returned —
/// a cancellation raised anywhere else travels in an output this never sees.
fn without_cancelled(mut output: RouterOutput, message_id: &[u8; 32]) -> RouterOutput {
    output.events.retain(|event| {
        !matches!(
            event,
            RouterEvent::MessageState { message_id: id, state: MessageState::Cancelled }
                if id == message_id
        )
    });
    output
}
