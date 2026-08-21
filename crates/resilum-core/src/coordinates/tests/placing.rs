use super::*;

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
fn the_last_mile_every_node_carries_does_not_inflate_a_fast_link() {
    let coordinates = Coordinates::default();

    settle(&coordinates, NEAR, ms(5), somewhere([0.005, 0.0, 0.0]));

    let estimate = coordinates.estimated_rtt(&NEAR).expect("placed");
    assert!(estimate < ms(10), "{estimate:?}");
}

#[test]
fn a_peer_not_heard_from_since_the_cutoff_is_forgotten() {
    let coordinates = Coordinates::default();
    settle(&coordinates, NEAR, ms(20), somewhere([0.02, 0.0, 0.0]));

    coordinates.forget_before(41.0);

    assert_eq!(coordinates.estimated_rtt(&NEAR), None);
}
