use std::collections::HashMap;

use super::lxmf;
use super::model::{
    CoordinatesStatus, Interface, NodeStatus, PlacedPeer, TorStatus, Transport, added_by, hex,
};
use crate::node::ResilumNode;
use resilum_core::discovery::Service;

/// Filled under `PathTableEntry::interface_index`, read back under
/// `InterfaceStats::interface_id.0` — two engine APIs naming the same
/// engine-assigned integer, checked by nothing. Should they diverge, every
/// lookup misses and interfaces report no peers instead of failing.
type PeersByInterface = HashMap<usize, Vec<String>>;

pub(super) fn snapshot(node: &ResilumNode) -> NodeStatus {
    let mut status = NodeStatus {
        running: node.0.is_running(),
        socks_port: node.0.socks_port(),
        identity_hash: None,
        reachable_destinations: 0,
        interfaces: Vec::new(),
        transport: None,
        nostr_relays: node
            .0
            .discovered(Service::NOSTR_RELAY)
            .iter()
            .map(hex)
            .collect(),
        lxmf: lxmf::snapshot(node),
        tor: node
            .0
            .tor_bootstrapped()
            .map(|bootstrapped| TorStatus { bootstrapped }),
        coordinates: CoordinatesStatus {
            ours: node.0.own_coordinate(),
            peers: node
                .0
                .placed_peers()
                .into_iter()
                .map(|placed| PlacedPeer {
                    identity_hash: hex(&placed.peer),
                    at: placed.at,
                    estimated_rtt_ms: placed.estimated_rtt.as_millis(),
                })
                .collect(),
        },
    };
    let Some(engine) = node.0.engine() else {
        return status;
    };
    status.identity_hash = Some(hex(&engine.identity_hash()));
    status.reachable_destinations = engine.path_count();

    let mut peer_hashes: PeersByInterface = HashMap::new();
    let mut peer_nodes: PeersByInterface = HashMap::new();
    for p in engine.path_table_entries() {
        if p.hops != 1 {
            continue;
        }
        peer_hashes
            .entry(p.interface_index)
            .or_default()
            .push(hex(&p.hash));
        if let Some(id) = engine.get_identity(&p.hash.into()) {
            let peer_node = hex(id.hash());
            let nodes = peer_nodes.entry(p.interface_index).or_default();
            if !nodes.contains(&peer_node) {
                nodes.push(peer_node);
            }
        }
    }

    status.interfaces = engine
        .interface_stats()
        .into_iter()
        .map(|i| Interface {
            added_by: added_by(&i.name),
            kind: i.kind.as_str(),
            discovered_via: node
                .0
                .discovered_via(i.interface_id)
                .unwrap_or_else(|| "direct".into()),
            peer_nodes: peer_nodes
                .get(&i.interface_id.0)
                .cloned()
                .unwrap_or_default(),
            peer_hashes: peer_hashes
                .get(&i.interface_id.0)
                .cloned()
                .unwrap_or_default(),
            name: i.name,
            online: i.online,
            local_client: i.is_local_client,
            rx_bytes: i.rx_bytes,
            tx_bytes: i.tx_bytes,
            bitrate: i.configured_bitrate,
        })
        .collect();
    let t = engine.transport_stats();
    status.transport = Some(Transport {
        packets_sent: t.packets_sent(),
        packets_received: t.packets_received(),
        packets_forwarded: t.packets_forwarded(),
        packets_dropped: t.packets_dropped(),
        announces_processed: t.announces_processed(),
    });
    status
}
