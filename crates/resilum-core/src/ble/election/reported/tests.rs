use super::super::HostsWhileOnARouter;
use super::{Facts, WhatThePlatformKnows};

#[test]
fn what_only_a_platform_could_know_stays_unset_until_one_says_so() {
    let known = WhatThePlatformKnows::nothing_yet();

    let read = known.read(0);

    assert!(!read.has_an_uplink);
    assert_eq!(read.p2p_and_sta_at_once, HostsWhileOnARouter::default());
}

#[test]
fn the_neighbours_we_counted_are_ours_not_the_platforms_to_report() {
    let known = WhatThePlatformKnows::nothing_yet();
    known.report(
        Facts {
            has_an_uplink: true,
            neighbours_heard: 99,
            ..Facts::default()
        },
        false,
    );

    let read = known.read(4);

    assert!(read.has_an_uplink);
    assert_eq!(read.neighbours_heard, 4);
}

#[test]
fn a_later_report_replaces_the_earlier_one() {
    let known = WhatThePlatformKnows::nothing_yet();
    known.report(
        Facts {
            has_an_uplink: true,
            ..Facts::default()
        },
        false,
    );
    known.report(Facts::default(), false);

    assert!(!known.read(0).has_an_uplink);
}

#[test]
fn a_host_that_can_read_its_own_power_does_not_wait_to_be_told() {
    let known = WhatThePlatformKnows::nothing_yet();
    known.report(
        Facts {
            charging: false,
            battery_percent: 0,
            ..Facts::default()
        },
        false,
    );

    let read = known.read(0);

    assert_eq!(read, super::super::what_this_host_can_answer(read));
}

#[test]
fn a_host_configured_to_host_stands_until_a_platform_says_it_cannot() {
    let known = WhatThePlatformKnows::unless_a_platform_says(true);

    assert!(known.can_host_at_all());

    known.report(Facts::default(), false);

    assert!(!known.can_host_at_all());
}
