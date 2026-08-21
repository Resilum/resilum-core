use super::*;

fn plausible() -> Claimed {
    Claimed {
        position: [1.0, 2.0, 3.0],
        height: 0.01,
        error: 0.5,
    }
}

#[test]
fn a_plausible_coordinate_is_believed() {
    assert!(plausible().believable().is_some());
}

#[test]
fn a_coordinate_carrying_nan_is_refused_before_violin_can_panic_on_it() {
    for poisoned in [
        Claimed {
            position: [f64::NAN, 0.0, 0.0],
            ..plausible()
        },
        Claimed {
            height: f64::NAN,
            ..plausible()
        },
        Claimed {
            error: f64::NAN,
            ..plausible()
        },
    ] {
        assert!(poisoned.believable().is_none(), "{poisoned:?}");
    }
}

#[test]
fn an_infinite_coordinate_is_refused() {
    assert!(
        Claimed {
            position: [f64::INFINITY, 0.0, 0.0],
            ..plausible()
        }
        .believable()
        .is_none()
    );
}

#[test]
fn a_peer_claiming_impossible_certainty_is_believed_no_further_than_the_floor() {
    for lie in [0.0, -1.0, 0.0001] {
        let believed = Claimed {
            error: lie,
            ..plausible()
        }
        .believable()
        .expect("a coordinate is not refused over its error alone");

        assert_eq!(believed.error_estimate(), LEAST_ERROR, "error {lie}");
    }
}

#[test]
fn a_peer_claiming_to_know_nothing_is_believed_no_further_than_the_ceiling() {
    let believed = Claimed {
        error: 900.0,
        ..plausible()
    }
    .believable()
    .expect("a coordinate is not refused over its error alone");

    assert_eq!(believed.error_estimate(), MOST_ERROR);
}

#[test]
fn a_coordinate_further_out_than_the_space_allows_is_refused() {
    assert!(
        Claimed {
            position: [FURTHEST + 1.0, 0.0, 0.0],
            ..plausible()
        }
        .believable()
        .is_none()
    );
}

#[test]
fn a_height_outside_what_a_way_into_a_mesh_costs_is_believed_no_further_than_the_bound() {
    let dug_in = Claimed {
        height: -0.001,
        ..plausible()
    }
    .believable()
    .expect("a coordinate is not refused over its height alone");
    let towering = Claimed {
        height: 90.0,
        ..plausible()
    }
    .believable()
    .expect("a coordinate is not refused over its height alone");

    assert_eq!(dug_in.height(), 0.0);
    assert_eq!(towering.height(), MOST_LAST_MILE);
}
