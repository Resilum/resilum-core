mod marking;
mod scale;

use resilum_core::coordinates::{Claimed, Coordinates};
use resilum_core::status::{BleStatus, CoordinatesStatus, Link, NodeStatus, PlacedPeer};

use super::super::tests::without_colour;
use super::{US, letter, of, quickest_way_to};

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
        without_colour(line)
            .chars()
            .position(|c| c == mark)
            .map(|column| (row, column))
    })
}

fn linked_at(nth: usize, at: [f64; 3]) -> (PlacedPeer, Link) {
    let who = format!("peer{nth}");
    (
        PlacedPeer {
            identity_hash: who.clone(),
            at: sitting_at(at),
            estimated_rtt_ms: 10,
        },
        Link {
            identity_hash: who,
            transport: "tor".to_owned(),
            interface_name: None,
            estimated_rtt_ms: Some(10),
        },
    )
}
