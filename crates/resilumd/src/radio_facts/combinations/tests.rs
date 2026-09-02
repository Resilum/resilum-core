use resilum_core::ble::election::HostsWhileOnARouter;

use super::{Combination, Role, read_from};

fn station_and_ap(channels_at_once: u32) -> Combination {
    Combination {
        each_limit_allows: vec![vec![Role::OnARouter], vec![Role::Hosting]],
        interfaces_at_once: 3,
        channels_at_once,
    }
}

#[test]
fn a_radio_that_names_no_access_point_cannot_host() {
    let station_only = Combination {
        each_limit_allows: vec![vec![Role::OnARouter]],
        interfaces_at_once: 2,
        channels_at_once: 2,
    };

    let allows = read_from(&[station_only]);

    assert!(!allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Refused);
}

#[test]
fn a_radio_that_holds_both_on_two_channels_keeps_its_router() {
    let allows = read_from(&[station_and_ap(2)]);

    assert!(allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Confirmed);
}

#[test]
fn a_radio_pinned_to_one_channel_would_have_to_leave_its_router() {
    let allows = read_from(&[station_and_ap(1)]);

    assert!(allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Refused);
}

#[test]
fn a_radio_with_room_for_one_interface_cannot_hold_both() {
    let alone = Combination {
        interfaces_at_once: 1,
        ..station_and_ap(2)
    };

    let allows = read_from(&[alone]);

    assert!(allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Refused);
}

#[test]
fn the_best_of_several_combinations_is_the_answer() {
    let allows = read_from(&[station_and_ap(1), station_and_ap(2)]);

    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Confirmed);
}
