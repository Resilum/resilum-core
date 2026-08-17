//! The subscriber ceiling, and that a refresh never trips it.

use super::*;

#[test]
fn a_bridge_stops_taking_subscribers_at_its_ceiling() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    for pubkey in 0..MAX_SUBSCRIBERS {
        let taken = registry.accept(nth(pubkey as u8, 100), 100);
        assert!(taken.is_ok(), "subscriber {pubkey} of {MAX_SUBSCRIBERS}");
    }

    let refused = registry.accept(nth(MAX_SUBSCRIBERS as u8, 100), 100);

    assert_eq!(
        refused,
        Err(AcceptError::Full {
            limit: MAX_SUBSCRIBERS
        }),
        "one past the ceiling was still taken"
    );
}

/// Subscribers refresh on a timer far shorter than the retention window. If
/// a keepalive counted as a new subscriber, a full bridge would start turning
/// away everyone it already carries.
#[test]
fn a_refresh_from_a_subscriber_already_held_is_not_turned_away() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    for pubkey in 0..MAX_SUBSCRIBERS {
        registry.accept(nth(pubkey as u8, 100), 100).expect("taken");
    }

    assert!(registry.accept(nth(0, 200), 200).is_ok());
    assert!(
        registry
            .accept(nth(MAX_SUBSCRIBERS as u8, 200), 200)
            .is_err()
    );
}

#[test]
fn a_subscriber_nobody_refreshed_stops_holding_a_slot() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    for pubkey in 0..MAX_SUBSCRIBERS {
        registry.accept(nth(pubkey as u8, 100), 100).expect("taken");
    }

    assert!(
        registry
            .accept(nth(MAX_SUBSCRIBERS as u8, 1000), 1000)
            .is_ok()
    );
}
