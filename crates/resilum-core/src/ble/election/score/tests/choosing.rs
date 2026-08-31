use super::{candidate, plain};
use crate::ble::election::score::{Candidate, Facts, HostsWhileOnARouter, the_one_to_host};

#[test]
fn a_tie_is_broken_by_the_lower_identity_so_both_sides_agree() {
    let lower = candidate(1, plain());
    let higher = candidate(2, plain());

    assert_eq!(the_one_to_host(&[lower, higher]), Some(lower));
    assert_eq!(the_one_to_host(&[higher, lower]), Some(lower));
}

#[test]
fn nobody_to_host_is_answered_with_nobody() {
    assert_eq!(the_one_to_host(&[]), None);
}

#[test]
fn a_device_that_cannot_host_never_wins_however_good_it_otherwise_looks() {
    let iphone = Candidate {
        can_host_at_all: false,
        ..candidate(
            1,
            Facts {
                has_an_uplink: true,
                p2p_and_sta_at_once: HostsWhileOnARouter::Confirmed,
                charging: true,
                battery_percent: 100,
                neighbours_heard: u8::MAX,
            },
        )
    };
    let a_plain_android = candidate(2, plain());

    assert_eq!(
        the_one_to_host(&[iphone, a_plain_android]),
        Some(a_plain_android)
    );
    assert_eq!(the_one_to_host(&[iphone]), None);
}
