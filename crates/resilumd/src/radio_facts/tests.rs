use resilum_core::ble::election::HostsWhileOnARouter;

use super::{WhatTheRadioAllows, whether_hosting_keeps_the_uplink};

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
fn a_host_that_cannot_name_its_radio_does_not_claim_a_free_uplink() {
    assert_eq!(
        whether_hosting_keeps_the_uplink(A_RADIO_THAT_CANNOT_DO_BOTH, Some("end0"), None),
        HostsWhileOnARouter::Refused
    );
}
