//! Node state snapshot for the caller, as a JSON string over the C ABI.
//!
//! JSON keeps the wire schema evolvable without touching the C ABI.

mod build;
mod lxmf;
mod model;
#[cfg(test)]
mod tests;

use std::ffi::CString;
use std::os::raw::c_char;

use crate::guard;
use crate::node::ResilumNode;
use crate::set_error;

/// A JSON snapshot of node state, or null on error with the reason in
/// `resilum_last_error`. Free with `resilum_string_free`. Shape:
/// ```json
/// {
///   "running": true,
///   "socks_port": 0 | null,
///   "identity_hash": "<32-hex>" | null,
///   "reachable_destinations": 0,
///   "interfaces": [
///     { "name": "<string>",
///       "added_by": "autoconnect" | "bootstrap" | "other",
///       "kind": "tcp" | "udp" | "i2p" | "serial" | "rnode" | "...",
///       "discovered_via": "tor" | "i2p" | "yggdrasil" | "iroh" | "covert"
///                       | "udp" | "ble" | "direct",
///       "online": true, "local_client": false,
///       "rx_bytes": 0, "tx_bytes": 0, "bitrate": 0 | null,
///       "peer_nodes": ["<32-hex>"], "peer_hashes": ["<32-hex>"] }
///   ],
///   "transport": { "packets_sent": 0, "packets_received": 0,
///                  "packets_forwarded": 0, "packets_dropped": 0,
///                  "announces_processed": 0 } | null,
///   "nostr_relays": ["<32-hex>"],
///   "lxmf": { "ready": true, "address": "<32-hex>", "queued_count": 0,
///             "queued_ids": ["<64-hex>"],
///             "propagation_node": "<32-hex>" | null } | null,
///   "tor": { "bootstrapped": true } | null,
///   "coordinates": {
///     "ours": { "position": [0.0, 0.0, 0.0], "height": 0.0, "error": 0.0 },
///     "peers": [ { "identity_hash": "<32-hex>",
///                  "at": { "position": [0.0, 0.0, 0.0],
///                          "height": 0.0, "error": 0.0 },
///                  "estimated_rtt_ms": 0 } ]
///   },
///   "ble": { "hosting_the_group": false },
///   "links": [ { "identity_hash": "<32-hex>",
///                "transport": "tor" | "i2p" | "yggdrasil" | "udp" | "ble"
///                            | "covert/icmp"
///                            | "...",
///                "interface_name": "<string>" | null,
///                "estimated_rtt_ms": 0 | null } ]
/// }
/// ```
/// `identity_hash` and `transport` are `null` before start. `added_by` is
/// inferred from `name`. `nostr_relays` are Nostr bridge LXMF addresses heard
/// on the mesh. `socks_port` is `null` when `ingress` names no `listen_tcp`:
/// discovery and probing run, but nothing local is bound and nothing will be.
///
/// `kind` and `discovered_via` share the word `i2p` but answer different
/// questions: `kind` is the socket the engine opened (a peer found over I2P is
/// still dialed as `tcp`), while `discovered_via` is the overlay that produced
/// the address — the one to colour a link by. `direct` means this crate did
/// not attach it: a bootstrap anchor, a LAN neighbour, or the engine's own.
///
/// `links` are the resilum peers this node keeps a link with, one entry per
/// link — the edges a map draws from this node. They are the set the quota
/// keeps (nearest, plus a quarter reserved for the furthest, so the graph stays
/// navigable), across every transport and whatever the hop count: a peer four
/// hops away over Tor is a link here, while `interfaces[].peer_nodes` holds
/// only what sits one hop away and follows the single path RNS chose, so a
/// covert peer reachable another way never appears under its covert interface.
/// The same peer appears once per transport that carries a link to it.
///
/// `coordinates` place peers in latency space rather than on the ground:
/// distance is round-trip time, in seconds, and `error` is how much the node
/// holding that coordinate trusts it — near 1.5 while it is still settling,
/// small once it has. `peers` is sorted nearest first and holds only those
/// this node has both measured and heard a coordinate from. The space has no
/// fixed orientation, so a map drawn from it should be aligned to the previous
/// frame rather than to the axes.
///
/// `tor` is `null` when this node runs no embedded Tor. A network that blocks
/// Tor leaves `bootstrapped` false for as long as the node runs: the directory
/// fetch retries rather than failing, so nothing else reports it and a start
/// over a blocked network succeeds like any other.
///
/// `lxmf` is `null` when this node has no messaging configured — never an
/// absent key, so the caller can tell "messaging is off" from "this build
/// predates the field". Its `queued_count` is sampled once per engine tick, not
/// read live, so it can lag a just-submitted message by one call.
/// `lxmf.queued_ids` names the messages that count covers, from the same
/// sample — a caller that lost its own record of what it sent reconciles
/// against it rather than discarding every delivery event for an id it does not
/// recognise, and an id in it can be handed to
/// `resilum_lxmf_requeue_with_method`. `lxmf.propagation_node` is likewise
/// `null` rather than absent when the router has selected none — the state a
/// `propagated` send is refused in — and can turn non-null on a later call.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_status(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        let status = build::snapshot(node);
        let json = match serde_json::to_string(&status) {
            Ok(json) => json,
            Err(e) => {
                set_error(format!("status is not representable as JSON: {e}"));
                return std::ptr::null_mut();
            }
        };
        match CString::new(json) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                set_error("status JSON contains an interior NUL");
                std::ptr::null_mut()
            }
        }
    })
}
