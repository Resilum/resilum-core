use resilum_core::coordinates::{Claimed, Coordinates};
use resilum_core::status::{BleStatus, CoordinatesStatus, Link, NodeStatus, PlacedPeer};

use super::space::NUDGE;
use super::{US, of, quickest_way_to};

fn nobody_but_us() -> NodeStatus {
    NodeStatus {
        version: "9.9.9".to_owned(),
        running: true,
        socks_port: None,
        identity_hash: None,
        reachable_destinations: 0,
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

fn sitting_at(position: [f64; 3]) -> Claimed {
    serde_json::from_str(&format!(
        r#"{{"position":{position:?},"height":0.0001,"error":0.5}}"#
    ))
    .expect("the published shape of a coordinate")
}

fn spot_of(drawn: &[String], mark: char) -> Option<(usize, usize)> {
    drawn.iter().enumerate().find_map(|(row, line)| {
        line.chars()
            .position(|c| c == mark)
            .map(|column| (row, column))
    })
}

#[test]
fn a_node_alone_still_marks_itself() {
    let drawn = of(&nobody_but_us());

    assert!(spot_of(&drawn, US).is_some(), "{drawn:?}");
}

#[test]
fn a_peer_sitting_elsewhere_is_drawn_elsewhere() {
    let mut status = nobody_but_us();
    status.coordinates.peers = vec![
        PlacedPeer {
            identity_hash: "aabb".to_owned(),
            at: sitting_at([0.2, -0.15, 0.05]),
            estimated_rtt_ms: 40,
        },
        PlacedPeer {
            identity_hash: "ccdd".to_owned(),
            at: sitting_at([-0.2, 0.15, -0.05]),
            estimated_rtt_ms: 90,
        },
    ];

    let drawn = of(&status);

    let us = spot_of(&drawn, US).expect("this node is marked");
    let one = spot_of(&drawn, 'a').expect("the first peer is marked");
    let other = spot_of(&drawn, 'b').expect("the second peer is marked");

    let apart = one.1.abs_diff(other.1);
    assert!(apart > NUDGE.len(), "{apart} columns apart: {drawn:?}");
    assert!(
        (one.1 < us.1) != (other.1 < us.1),
        "peers sitting either side of us landed on one side: {drawn:?}"
    );
}

#[test]
fn a_peer_is_marked_in_the_ink_of_its_quickest_way_not_its_first() {
    let mut status = nobody_but_us();
    status.coordinates.peers = vec![PlacedPeer {
        identity_hash: "aabb".to_owned(),
        at: Coordinates::default().ours(),
        estimated_rtt_ms: 40,
    }];
    status.links = vec![
        Link {
            identity_hash: "aabb".to_owned(),
            transport: "covert_icmp".to_owned(),
            interface_name: None,
            estimated_rtt_ms: Some(900),
        },
        Link {
            identity_hash: "aabb".to_owned(),
            transport: "yggdrasil".to_owned(),
            interface_name: None,
            estimated_rtt_ms: Some(12),
        },
    ];

    assert_eq!(
        quickest_way_to(&status, "aabb").as_deref(),
        Some("yggdrasil")
    );
}
