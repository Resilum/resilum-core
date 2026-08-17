use super::model::{LxmfStatus, hex};
use crate::node::ResilumNode;

/// `None` when this node has no `lxmf` config section, or is not running —
/// the messaging handle only exists once it is.
///
/// Gated on the handle existing, not on the router being ready: the handle
/// is built at start, one engine tick before the router can register.
/// `address` comes straight from the identity, so it is already correct at
/// that point. Waiting for `ready` would leave the whole section `null` for
/// that first tick and then have it appear — worse for a startup screen than
/// showing the address (e.g. for a QR code) right away with `ready: false`
/// until the router catches up.
pub(super) fn snapshot(node: &ResilumNode) -> Option<LxmfStatus> {
    let lxmf = node.0.lxmf()?;
    // The accessor `resilum_lxmf_address` returns, not the handle's own copy:
    // snapshot and symbol are then one value.
    let address = crate::lxmf::address_hex(node)?;
    // Read once, then counted: two reads can straddle an engine tick.
    let queued = lxmf.queued_ids();
    Some(LxmfStatus {
        ready: lxmf.is_ready(),
        address,
        queued_count: queued.len() as u64,
        queued_ids: queued.iter().map(hex).collect(),
        propagation_node: lxmf.propagation_node().map(|hash| hex(&hash)),
    })
}
