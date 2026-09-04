use resilum_core::ble::election::HostsWhileOnARouter;

use super::{Combination, Limit, Role, read_from};

fn one_of(role: Role) -> Limit {
    Limit {
        allows: vec![role],
        at_most: 1,
    }
}

fn station_and_ap(channels_at_once: u32) -> Combination {
    Combination {
        each_limit: vec![one_of(Role::OnARouter), one_of(Role::Hosting)],
        interfaces_at_once: 3,
        channels_at_once,
    }
}

fn either_but_only_one(at_most: u32) -> Combination {
    Combination {
        each_limit: vec![Limit {
            allows: vec![Role::OnARouter, Role::Hosting],
            at_most,
        }],
        interfaces_at_once: 3,
        channels_at_once: 2,
    }
}

#[test]
fn a_radio_that_names_no_access_point_cannot_host() {
    let station_only = Combination {
        each_limit: vec![one_of(Role::OnARouter)],
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
fn one_slot_shared_between_the_two_roles_holds_only_one_of_them() {
    let allows = read_from(&[either_but_only_one(1)]);

    assert!(allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Refused);
}

#[test]
fn a_shared_slot_wide_enough_for_two_holds_both() {
    let allows = read_from(&[either_but_only_one(2)]);

    assert!(allows.can_host_at_all);
    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Confirmed);
}

#[test]
fn the_best_of_several_combinations_is_the_answer() {
    let allows = read_from(&[station_and_ap(1), station_and_ap(2)]);

    assert_eq!(allows.while_on_a_router, HostsWhileOnARouter::Confirmed);
}
