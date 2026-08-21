mod placing;

use super::beginning::SCATTERED_WITHIN;
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
fn a_fresh_node_begins_a_plausible_round_trip_from_the_origin_not_a_whole_second() {
    let ours = Coordinates::default().ours();

    for axis in ours.position {
        assert!(axis.abs() <= SCATTERED_WITHIN, "{axis} is a whole second");
    }
}

#[test]
fn a_last_mile_that_ran_away_is_trimmed_to_what_a_way_into_a_mesh_can_cost() {
    let mut node: Node<Space, Adjustments> = Node::new();
    let mut towering = Coord::<Space>::from([0.01, 0.0, 0.0]);
    towering.set_height(5.0);
    node.set_coordinate(towering);

    let trimmed = without_a_runaway_last_mile(&node);

    assert_eq!(trimmed.height(), claimed::MOST_LAST_MILE);
}

#[test]
fn only_reaching_a_peer_slowly_grows_a_leg_rather_than_moving_us_away_from_everyone() {
    let coordinates = Coordinates::default();

    for round in 0..40 {
        coordinates.believe(
            NEAR,
            ONE_LINK,
            ms(20),
            somewhere([0.02, 0.0, 0.0]),
            f64::from(round),
        );
        coordinates.believe(
            FAR,
            ONE_LINK,
            ms(900),
            somewhere([0.03, 0.0, 0.0]),
            f64::from(round),
        );
    }

    assert!(
        coordinates.ours().height() > 0.0,
        "{:?}",
        coordinates.ours()
    );
}

#[test]
fn the_error_a_peer_is_told_is_bounded_where_our_own_is_not() {
    let coordinates = Coordinates::default();

    for round in 0..10 {
        coordinates.believe(
            NEAR,
            ONE_LINK,
            ms(1),
            somewhere([5.0, 0.0, 0.0]),
            f64::from(round),
        );
    }

    assert!(coordinates.how_wrong_we_are() > claimed::MOST_ERROR);
    assert_eq!(coordinates.ours().error(), claimed::MOST_ERROR);
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
