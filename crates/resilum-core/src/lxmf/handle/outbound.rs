//! What the application does to the outbound queue, and what it can read back
//! of it.

use leviculum_lxmf::{DeliveryMethod, Message};

use super::{Command, LxmfHandle};
use crate::error::Result;

impl LxmfHandle {
    /// How many messages are queued for delivery, including the ones restored
    /// from a checkpoint at start. Sampled once per engine tick, so it lags a
    /// [`Self::submit`] by up to one.
    #[must_use]
    pub fn outbound_depth(&self) -> usize {
        self.router_state.outbound_depth()
    }

    /// The ids [`Self::outbound_depth`] counts, from the same sample, so the
    /// two always agree. A caller that lost its own record of what it sent —
    /// reinstalled, or restored onto another device — reconciles against this
    /// instead of discarding every delivery event it does not recognise.
    ///
    /// The checkpoint stores the packed message an id was derived from, so an
    /// id here is the same one [`Self::submit`]'s caller was given however many
    /// restarts ago.
    #[must_use]
    pub fn queued_ids(&self) -> Vec<[u8; 32]> {
        self.router_state.queued_ids()
    }

    /// Returns once queued, not once sent; delivery arrives through
    /// [`LxmfHandle::next_event`].
    pub fn submit(&self, message: Message) -> Result<()> {
        self.send(Command::Send(Box::new(message)))
    }

    /// Re-queue a message the router is still carrying, under `method` instead
    /// of the one it was submitted with — a direct send nobody answered moved
    /// onto a propagation node, say.
    ///
    /// The id does not change: a message id covers the destination, source and
    /// payload, and no part of the delivery method. That is also why
    /// [`Self::submit`] cannot do this job — the router refuses the second copy
    /// as a duplicate of the one still queued.
    ///
    /// Returns once the request is queued, not once it is applied: a router
    /// that no longer held the message says so through
    /// [`LxmfHandle::next_event`] as a `requeue_refused` event.
    pub fn requeue_with_method(&self, message_id: [u8; 32], method: DeliveryMethod) -> Result<()> {
        self.send(Command::Requeue { message_id, method })
    }
}
