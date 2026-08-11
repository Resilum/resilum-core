//! The shape the app decodes.

use serde::Serialize;

#[derive(Serialize)]
pub(super) struct NodeStatus {
    pub running: bool,
    pub socks_port: u16,
    /// This node's RNS identity hash (hex), or null before start.
    pub identity: Option<String>,
    /// Known reachable destinations on the mesh.
    pub path_count: usize,
    pub interfaces: Vec<Interface>,
    pub transport: Option<Transport>,
}

#[derive(Serialize)]
pub(super) struct Interface {
    pub name: String,
    pub source: &'static str,
    /// Transport medium (tcp/udp/i2p/serial/rnode/…), from the interface the
    /// engine built — not inferred from the name. Group the UI by this.
    pub kind: &'static str,
    /// Overlay the peer was discovered through (tor/i2p/yggdrasil/covert), or
    /// `direct` when resilum-core did not attach it. Orthogonal to `kind`: a
    /// peer found over I2P is still dialed as `tcp`.
    pub discovered_via: String,
    pub online: bool,
    pub local_client: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub bitrate: Option<u32>,
    /// Identity hashes (hex) of the node(s) reached one hop over this interface.
    /// The grouping key for "same node across overlays": a node's destinations
    /// spread across its interfaces but all resolve to its one identity.
    pub peer_nodes: Vec<String>,
    /// The one-hop destination hashes (hex) `peer_nodes` resolved from. Detail
    /// only — they distribute across a node's interfaces, so they don't group.
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

pub(super) fn interface_source(name: &str) -> &'static str {
    if name.starts_with("autoconnect") {
        "autoconnect"
    } else if name.starts_with("tcp_client") {
        "bootstrap"
    } else {
        "other"
    }
}

pub(super) fn hex16(bytes: &[u8; 16]) -> String {
    let mut s = String::with_capacity(32);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}
