use resilum_core::coordinates::Coordinates;
use resilum_core::status::{Link, PlacedPeer};

use super::super::space::NUDGE;
use super::{
    US, letter, linked_at, nobody_but_us, of, quickest_way_to, sitting_at, spot_of, without_colour,
};

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
fn peers_crowded_together_all_keep_their_marks_however_the_labels_fall() {
    let mut status = nobody_but_us();
    status.coordinates.ours = sitting_at([0.824, 0.570, 0.0]);
    let crowded = [[-0.078, -0.063], [-0.100, -0.067], [-0.190, -0.127]];
    status.coordinates.peers = crowded
        .iter()
        .enumerate()
        .map(|(nth, at)| PlacedPeer {
            identity_hash: format!("peer{nth}"),
            at: sitting_at([at[0], at[1], 0.0]),
            estimated_rtt_ms: 1000 + nth as u128,
        })
        .collect();

    let drawn = of(&status);

    for nth in 0..crowded.len() {
        let mark = letter(nth);
        assert!(
            spot_of(&drawn, mark).is_some(),
            "{mark} is missing: {drawn:?}"
        );
    }
}

#[test]
fn a_time_goes_after_its_own_mark_while_there_is_room_so_it_is_not_read_as_the_neighbours() {
    let mut status = nobody_but_us();
    for (nth, along) in [0.05, 0.10].into_iter().enumerate() {
        let (peer, link) = linked_at(nth, [along, 0.0, 0.0]);
        status.coordinates.peers.push(peer);
        status.links.push(link);
    }

    let drawn = of(&status);

    let (row, column) = spot_of(&drawn, letter(0)).expect("the peer is marked");
    let after: String = without_colour(&drawn[row])
        .chars()
        .skip(column + 1)
        .collect();
    assert!(after.starts_with(" 10 ms"), "{drawn:?}");
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
