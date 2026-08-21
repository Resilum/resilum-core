use super::*;

const NEAR: PeerId = [1; 16];
const FAR: PeerId = [2; 16];
const ONE_LINK: LinkId = 1;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn settle(coordinates: &Coordinates, peer: PeerId, rtt: Duration, theirs: Claimed) {
    for round in 0..40 {
        assert!(coordinates.believe(peer, ONE_LINK, rtt, theirs, f64::from(round)));
    }
}

fn somewhere(position: [f64; 3]) -> Claimed {
    Claimed {
        position,
        height: 0.001,
        error: 0.5,
    }
}

#[test]
fn a_fresh_node_admits_it_does_not_know_where_it_is() {
    assert_eq!(Coordinates::default().ours().error, claimed::MOST_ERROR);
}

/// Coordinates on the same spot have no direction between them, and `violin`
/// answers that with the first axis: a mesh that all began at the origin lays
/// itself out along a line, whatever the latencies say.
#[test]
fn two_fresh_nodes_do_not_begin_on_the_same_spot() {
    let ours = Coordinates::default().ours();
    let theirs = Coordinates::default().ours();

    assert_ne!(ours.position, theirs.position);
    assert_ne!(ours.position, [0.0, 0.0, 0.0]);
}

#[test]
fn a_node_that_has_measured_nobody_places_nobody() {
    let coordinates = Coordinates::default();

    assert_eq!(coordinates.estimated_rtt(&NEAR), None);
    assert!(coordinates.nearest(10).is_empty());
}

#[test]
fn the_peer_measured_closer_is_estimated_closer() {
    let coordinates = Coordinates::default();

    settle(&coordinates, NEAR, ms(20), somewhere([0.02, 0.0, 0.0]));
    settle(&coordinates, FAR, ms(400), somewhere([0.0, 0.4, 0.0]));

    let near = coordinates.estimated_rtt(&NEAR).expect("placed");
    let far = coordinates.estimated_rtt(&FAR).expect("placed");
    assert!(near < far, "near {near:?} is not closer than far {far:?}");
    let nearest = coordinates.nearest(1);
    assert_eq!(nearest.len(), 1);
    assert_eq!(nearest[0].peer, NEAR);
    assert_eq!(nearest[0].estimated_rtt, near);
}

#[test]
fn a_coordinate_this_node_will_not_believe_takes_its_sample_with_it() {
    let coordinates = Coordinates::default();
    let poisoned = Claimed {
        position: [f64::NAN, 0.0, 0.0],
        height: 0.0,
        error: 0.5,
    };

    assert!(!coordinates.believe(NEAR, ONE_LINK, ms(20), poisoned, 1.0));

    assert_eq!(coordinates.estimated_rtt(&NEAR), None);
}

/// Our own coordinate is the one thing a peer must not be able to move
/// somewhere it cannot come back from.
#[test]
fn our_own_coordinate_stays_finite_whatever_arrives() {
    let coordinates = Coordinates::default();

    settle(&coordinates, NEAR, ms(20), somewhere([0.02, 0.0, 0.0]));
    for round in 0..40 {
        coordinates.believe(
            FAR,
            ONE_LINK,
            ms(1),
            somewhere([600.0, -600.0, 600.0]),
            f64::from(round),
        );
    }

    let ours = coordinates.ours();
    assert!(ours.believable().is_some(), "{ours:?}");
}

#[test]
fn a_peer_not_heard_from_since_the_cutoff_is_forgotten() {
    let coordinates = Coordinates::default();
    settle(&coordinates, NEAR, ms(20), somewhere([0.02, 0.0, 0.0]));

    coordinates.forget_before(41.0);

    assert_eq!(coordinates.estimated_rtt(&NEAR), None);
}
