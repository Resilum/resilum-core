//! The shape the consumer decodes. `resilum_node_status`'s doc comment is the
//! published copy of it — it is what reaches a caller holding only the header.

use resilum_core::coordinates::Claimed;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct NodeStatus {
    pub running: bool,
    pub socks_port: Option<u16>,
    pub identity_hash: Option<String>,
    pub reachable_destinations: usize,
    pub interfaces: Vec<Interface>,
    pub transport: Option<Transport>,
    pub nostr_relays: Vec<String>,
    pub lxmf: Option<LxmfStatus>,
    pub tor: Option<TorStatus>,
    pub coordinates: CoordinatesStatus,
}

#[derive(Serialize)]
pub(super) struct CoordinatesStatus {
    pub ours: Claimed,
    pub peers: Vec<PlacedPeer>,
}

#[derive(Serialize)]
pub(super) struct PlacedPeer {
    pub identity_hash: String,
    pub at: Claimed,
    pub estimated_rtt_ms: u128,
}

#[derive(Serialize)]
pub(super) struct TorStatus {
    pub bootstrapped: bool,
}

#[derive(Serialize)]
pub(super) struct LxmfStatus {
    /// Permanently false if the processor panicked and was detached.
    pub ready: bool,
    pub address: String,
    pub queued_count: u64,
    /// An id survives a restart unchanged.
    pub queued_ids: Vec<String>,
    pub propagation_node: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Interface {
    pub name: String,
    pub added_by: &'static str,
    pub kind: &'static str,
    pub discovered_via: String,
    pub online: bool,
    pub local_client: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub bitrate: Option<u32>,
    /// Identity hashes, 32 hex chars each, of the node(s) reached one hop over
    /// this interface. The grouping key for "same node across overlays": a
    /// node's destinations spread across its interfaces but all resolve to its
    /// one identity.
    pub peer_nodes: Vec<String>,
    /// The one-hop destination hashes, 32 hex chars each, `peer_nodes` resolved
    /// from. Detail only — they distribute across a node's interfaces, so they
    /// don't group.
    pub peer_hashes: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct Transport {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_forwarded: u64,
    pub packets_dropped: u64,
    pub announces_processed: u64,
}

pub(super) fn added_by(name: &str) -> &'static str {
    if name.starts_with("autoconnect") {
        "autoconnect"
    } else if name.starts_with("tcp_client") {
        "bootstrap"
    } else {
        "other"
    }
}

pub(super) fn hex<const N: usize>(bytes: &[u8; N]) -> String {
    resilum_core::hex::encode(bytes.iter())
}
