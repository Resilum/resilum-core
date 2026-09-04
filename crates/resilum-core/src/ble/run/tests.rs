use super::{A_SCAN_THAT_HEARD_NOTHING_IS_RESTARTED_AFTER_MS, a_scan_this_quiet_may_have_died};

const ANDROID_COUNTS_STARTS_OVER_MS: u64 = 30_000;
const AND_ALLOWS_THIS_MANY: u64 = 5;

#[test]
fn a_scan_that_is_hearing_peers_is_left_alone() {
    assert!(!a_scan_this_quiet_may_have_died(5_000, 4_000));
}

#[test]
fn a_scan_that_has_gone_quiet_long_enough_is_started_again() {
    assert!(a_scan_this_quiet_may_have_died(
        A_SCAN_THAT_HEARD_NOTHING_IS_RESTARTED_AFTER_MS,
        0
    ));
}

#[test]
fn we_stay_under_the_starts_android_allows() {
    let starts_in_a_window =
        ANDROID_COUNTS_STARTS_OVER_MS / A_SCAN_THAT_HEARD_NOTHING_IS_RESTARTED_AFTER_MS;

    assert!(
        starts_in_a_window < AND_ALLOWS_THIS_MANY,
        "restarting this often is what stopped Android reporting results"
    );
}
