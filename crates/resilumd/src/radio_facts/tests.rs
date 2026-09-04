use resilum_core::ble::election::HostsWhileOnARouter;

use super::{
    WhatTheRadioAllows, the_group_would_take_our_way_out, whether_hosting_keeps_the_uplink,
};

const A_RADIO_THAT_CANNOT_DO_BOTH: WhatTheRadioAllows = WhatTheRadioAllows {
    can_host_at_all: true,
    while_on_a_router: HostsWhileOnARouter::Refused,
};

#[test]
fn a_wired_uplink_costs_nothing_when_we_host_on_the_radio() {
    assert_eq!(
        whether_hosting_keeps_the_uplink(A_RADIO_THAT_CANNOT_DO_BOTH, Some("end0"), Some("wlan0")),
        HostsWhileOnARouter::Confirmed
    );
}

#[test]
fn an_uplink_over_the_very_radio_we_would_host_on_is_the_one_at_risk() {
    assert_eq!(
        whether_hosting_keeps_the_uplink(A_RADIO_THAT_CANNOT_DO_BOTH, Some("wlan0"), Some("wlan0")),
        HostsWhileOnARouter::Refused
    );
}

#[test]
fn with_no_way_out_the_radio_has_the_only_word() {
    assert_eq!(
        whether_hosting_keeps_the_uplink(A_RADIO_THAT_CANNOT_DO_BOTH, None, Some("wlan0")),
        HostsWhileOnARouter::Refused
    );
}

#[test]
fn a_node_that_reaches_the_world_by_the_group_radio_would_lose_it() {
    assert!(the_group_would_take_our_way_out(
        Some("wlan0"),
        Some("wlan0")
    ));
}

#[test]
fn a_wired_node_gives_up_nothing_by_taking_part() {
    assert!(!the_group_would_take_our_way_out(
        Some("end0"),
        Some("wlan0")
    ));
}

#[test]
fn a_node_with_no_way_out_has_none_to_lose() {
    assert!(!the_group_would_take_our_way_out(None, Some("wlan0")));
}

#[test]
fn a_node_that_cannot_name_the_radio_assumes_the_worst() {
    assert!(the_group_would_take_our_way_out(Some("wlan0"), None));
}

#[test]
fn a_host_that_cannot_name_its_radio_does_not_claim_a_free_uplink() {
    assert_eq!(
        whether_hosting_keeps_the_uplink(A_RADIO_THAT_CANNOT_DO_BOTH, Some("end0"), None),
        HostsWhileOnARouter::Refused
    );
}
