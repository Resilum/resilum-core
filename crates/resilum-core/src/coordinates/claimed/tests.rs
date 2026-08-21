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

/// `violin` decides a coordinate is invalid only after moving ours with it,
/// and its distance is a `Duration`, which panics on a value like this.
#[test]
fn a_coordinate_carrying_nan_is_refused() {
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

/// Error weights how far one sample may move us, so a peer claiming to be
/// certain beyond what any node reaches would move us all the way.
#[test]
fn a_peer_claiming_impossible_certainty_is_refused() {
    for lie in [0.0, -1.0, 0.0001] {
        assert!(
            Claimed {
                error: lie,
                ..plausible()
            }
            .believable()
            .is_none(),
            "error {lie}"
        );
    }
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
fn a_negative_height_is_refused() {
    assert!(
        Claimed {
            height: -0.001,
            ..plausible()
        }
        .believable()
        .is_none()
    );
}
