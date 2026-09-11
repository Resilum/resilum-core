use resilum_core::status::PlacedPeer;

use super::{letter, linked_at, nobody_but_us, of, sitting_at, spot_of};

#[test]
fn one_peer_far_out_does_not_squeeze_the_rest_into_a_corner() {
    let mut status = nobody_but_us();
    for nth in 0..9 {
        let (peer, link) = linked_at(nth, [0.01 * nth as f64, 0.0, 0.0]);
        status.coordinates.peers.push(peer);
        status.links.push(link);
    }
    let (far_peer, far_link) = linked_at(9, [8.0, 8.0, 0.0]);
    status.coordinates.peers.push(far_peer);
    status.links.push(far_link);

    let drawn = of(&status);

    let spread: Vec<usize> = (0..9)
        .filter_map(|nth| spot_of(&drawn, letter(nth)).map(|spot| spot.1))
        .collect();
    let widest = spread.iter().max().unwrap_or(&0) - spread.iter().min().unwrap_or(&0);
    assert!(
        widest > 8,
        "the near ones sit within {widest} columns: {drawn:?}"
    );
}

#[test]
fn a_peer_we_hold_no_link_with_does_not_set_the_scale() {
    let mut status = nobody_but_us();
    let (held, link) = linked_at(0, [0.05, 0.0, 0.0]);
    status.coordinates.peers = vec![held];
    status.links = vec![link];
    let with_only_a_link = of(&status);

    status.coordinates.peers.push(PlacedPeer {
        identity_hash: "never-linked".to_owned(),
        at: sitting_at([9.0, 9.0, 0.0]),
        estimated_rtt_ms: 5000,
    });

    assert_eq!(
        spot_of(&of(&status), letter(0)),
        spot_of(&with_only_a_link, letter(0)),
    );
}
