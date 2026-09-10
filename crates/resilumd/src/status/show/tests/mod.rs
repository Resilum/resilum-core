mod fitting;
mod telling;

use resilum_core::coordinates::Coordinates;
use resilum_core::status::{BleStatus, CoordinatesStatus, Interface, Link, NodeStatus};

use super::Asked;

const PLAINLY: Asked = Asked {
    map: false,
    interfaces: true,
    links: true,
};

const IN_SHORT: Asked = Asked {
    map: false,
    interfaces: false,
    links: false,
};

fn a_node() -> NodeStatus {
    NodeStatus {
        version: "9.9.9".to_owned(),
        running: true,
        socks_port: None,
        identity_hash: Some("aabbccddeeff0011".to_owned()),
        reachable_destinations: 7,
        interfaces: Vec::new(),
        transport: None,
        nostr_relays: Vec::new(),
        lxmf: None,
        tor: None,
        coordinates: CoordinatesStatus {
            ours: Coordinates::default().ours(),
            peers: Vec::new(),
        },
        links: Vec::new(),
        ble: BleStatus {
            hosting_the_group: false,
        },
    }
}

fn a_link(transport: &str, over: Option<&str>) -> Link {
    Link {
        identity_hash: "1122334455667788".to_owned(),
        transport: transport.to_owned(),
        interface_name: over.map(str::to_owned),
        estimated_rtt_ms: Some(40),
    }
}

fn a_carrier(name: &str, online: bool) -> Interface {
    Interface {
        name: name.to_owned(),
        added_by: "other".to_owned(),
        kind: "tcp".to_owned(),
        discovered_via: "direct".to_owned(),
        online,
        local_client: false,
        rx_bytes: 2048,
        tx_bytes: 0,
        bitrate: None,
        peer_nodes: Vec::new(),
        peer_hashes: Vec::new(),
    }
}

fn without_colour(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            plain.push(c);
            continue;
        }
        for escaped in chars.by_ref() {
            if escaped == 'm' {
                break;
            }
        }
    }
    plain
}
