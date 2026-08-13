//! Minting the delivery destination, and the state transitions around it.

use std::sync::atomic::Ordering;

use leviculum_lxmf::propagation_client::PropagationTransport;
use leviculum_lxmf::router::{LxmfRouter, PropagationClientConfig, RouterConfig};
use leviculum_lxmf::{LxmfNode, LxmfNodeConfig};
use leviculum_std::api::Identity;
use leviculum_std::driver::StdNodeCore;

use super::{LxmfProcessor, Ready, State};
use crate::lxmf::checkpoint::Checkpoint;

impl LxmfProcessor {
    /// Called from both hooks, not just the timer one: nothing in the seam
    /// promises which fires first, and an event handled before the router
    /// exists is an inbound message lost at startup.
    fn register_if_needed(&mut self, core: &mut StdNodeCore) {
        let identity = match std::mem::replace(&mut self.state, State::Failed) {
            State::Unregistered(identity) => *identity,
            other => {
                self.state = other;
                return;
            }
        };
        self.state = match register(core, identity, self.checkpoint.as_ref()) {
            Ok(ready) => {
                tracing::info!(
                    address = %crate::hex::encode(ready.delivery_hash.iter()),
                    "lxmf ready",
                );
                self.registered.store(true, Ordering::Relaxed);
                State::Ready(Box::new(ready))
            }
            Err(detail) => {
                tracing::error!(error = %detail, "lxmf delivery destination not registered");
                State::Failed
            }
        };
    }

    /// Moves the router out of `self` for the duration of the hook, so the rest
    /// of it can borrow the router and the event sink at once. A panic in
    /// between leaves the slot `Failed`, which costs nothing: the driver
    /// detaches a panicking processor permanently anyway.
    pub(super) fn take_ready(&mut self, core: &mut StdNodeCore) -> Option<Box<Ready>> {
        self.register_if_needed(core);
        match std::mem::replace(&mut self.state, State::Failed) {
            State::Ready(ready) => Some(ready),
            other => {
                self.state = other;
                None
            }
        }
    }
}

/// Built on the node's own identity, so the address peers message is the one
/// `Node::lxmf_address` reports before the node has started.
fn register(
    core: &mut StdNodeCore,
    identity: Identity,
    checkpoint: Option<&Checkpoint>,
) -> Result<Ready, String> {
    let identity_hash = *identity.hash();
    let destination = LxmfNode::delivery_destination(clone_identity(&identity)?)
        .map_err(|e| format!("delivery destination: {e:?}"))?;
    let delivery_hash = *destination.hash().as_bytes();
    let node = LxmfNode::register(core, destination, LxmfNodeConfig::default())
        .map_err(|e| format!("register delivery destination: {e:?}"))?;
    let mut router = LxmfRouter::new(node, identity_hash, RouterConfig::default());

    // Without the mailbox client a message only moves while both peers are
    // online at once.
    let mailbox = PropagationTransport::destination(clone_identity(&identity)?)
        .map_err(|e| format!("propagation destination: {e:?}"))?;
    let transport = PropagationTransport::register(core, mailbox)
        .map_err(|e| format!("register propagation destination: {e:?}"))?;
    router
        .enable_propagation_client(transport, PropagationClientConfig::default())
        .map_err(|e| format!("enable propagation client: {e:?}"))?;

    // Before anything else touches the router: the restored ids are what keep
    // a redelivered message from surfacing twice.
    if let Some(checkpoint) = checkpoint
        && let Err(e) = router.restore(checkpoint)
    {
        tracing::warn!(error = ?e, "lxmf checkpoint not restored");
    }
    Ok(Ready {
        router,
        delivery_hash,
        // Both on the first tick: nobody can message an unannounced
        // destination, and mail may have waited the whole downtime.
        next_announce_ms: 0,
        next_sync_ms: 0,
    })
}

/// `delivery_destination` and the mailbox both consume an identity, so each
/// needs its own copy of the private key.
fn clone_identity(identity: &Identity) -> Result<Identity, String> {
    let bytes = identity
        .private_key_bytes()
        .map_err(|e| format!("identity has no private key: {e:?}"))?;
    Identity::from_private_key_bytes(&bytes).map_err(|e| format!("could not copy identity: {e:?}"))
}
