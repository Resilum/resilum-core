//! The pieces of a network a propagated send needs, without a network.

use leviculum_core::DestinationHash;
use leviculum_core::traits::Storage as _;
use leviculum_lxmf::router::PropagationClientConfig;
use leviculum_lxmf::{KnownPropagationNode, PropagationNodeAnnounce, PropagationTransport};
use leviculum_std::api::Identity;
use leviculum_std::driver::StdNodeCore;

use super::super::Ready;

/// Announce a propagation node to the router's own client and select it:
/// `restore_known_node` is the path a restart takes, so the router reaches the
/// same state it would from a heard announce. Returns the node's hash, so a
/// caller can check the router selected exactly the one it announced.
pub(super) fn select_propagation_node(
    ready: &mut Ready,
    core: &mut StdNodeCore,
    stamp_cost: u64,
) -> DestinationHash {
    let identity = crate::identity::generate();
    let node =
        PropagationTransport::destination(copy_of(&identity)).expect("propagation destination");
    let destination = DestinationHash::new(*node.hash().as_bytes());
    // The router drops a known node whose identity or announce the core has
    // forgotten, and asks for a fresh announce instead. Both are what a heard
    // announce would have left behind.
    core.storage_mut()
        .set_identity(*destination.as_bytes(), identity);
    core.storage_mut().set_announce_cache(
        *destination.as_bytes(),
        announce(stamp_cost).encode().expect("announce app data"),
    );
    let mut transport = ready
        .router
        .disable_propagation_client()
        .expect("propagation client");
    transport.restore_known_node(KnownPropagationNode {
        destination,
        announce: announce(stamp_cost),
    });
    ready
        .router
        .enable_propagation_client(transport, PropagationClientConfig::default())
        .expect("re-enable propagation client");
    let _selection = ready
        .router
        .select_outbound_propagation_node(core, Some(destination))
        .expect("select propagation node");
    destination
}

fn announce(stamp_cost: u64) -> PropagationNodeAnnounce {
    PropagationNodeAnnounce {
        legacy_support: false,
        timebase: 0,
        enabled: true,
        transfer_limit_kb: 1_000,
        sync_limit_kb: 1_000,
        stamp_cost,
        stamp_cost_flexibility: 0,
        peering_cost: 0,
        metadata: Vec::new(),
    }
}

/// `Identity` is deliberately not `Clone`, and the core and the processor each
/// need their own.
pub(super) fn copy_of(identity: &Identity) -> Identity {
    let bytes = identity.private_key_bytes().expect("private key");
    Identity::from_private_key_bytes(&bytes).expect("copy identity")
}

/// Make a peer addressable: preparing the propagation envelope encrypts to its
/// public identity, which a node normally learns from an announce.
pub(super) fn known_peer(core: &mut StdNodeCore) -> [u8; 16] {
    let peer = crate::identity::generate();
    let address = crate::identity::lxmf_address(&peer);
    core.storage_mut().set_identity(address, peer);
    address
}
