use super::*;

const TOR: LinkId = 1;
const YGG: LinkId = 2;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_peer_nobody_has_measured_has_no_distance() {
    assert_eq!(Peer::default().fastest_link(), None);
}

#[test]
fn a_peer_reachable_two_ways_is_as_far_as_its_faster_way() {
    let mut peer = Peer::default();
    for _ in 0..3 {
        peer.measured(TOR, ms(900));
        peer.measured(YGG, ms(40));
    }

    assert_eq!(peer.fastest_link(), Some(ms(40)));
}

#[test]
fn a_peer_that_has_lost_its_faster_way_is_as_far_as_what_is_left() {
    let mut peer = Peer::default();
    for _ in 0..3 {
        peer.measured(TOR, ms(900));
        peer.measured(YGG, ms(40));
    }

    peer.forget_link(YGG);

    assert_eq!(peer.fastest_link(), Some(ms(900)));
}
