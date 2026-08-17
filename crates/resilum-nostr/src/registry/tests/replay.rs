//! Whether an accept, a refresh and an expiry hand out a delivery address —
//! and never a stale one.

use super::*;

#[test]
fn an_older_subscription_cannot_replace_a_newer_one() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    registry.accept(sub(200, 0xaa), 200).expect("first");

    assert_eq!(
        registry.accept(sub(100, 0xbb), 200),
        Err(AcceptError::Replayed)
    );
    assert_eq!(registry.lxmf_for(&[7u8; 32]).map(|a| a[15]), Some(0xaa));
}

/// A capture replayed unchanged carries the very timestamp of the record it
/// would overwrite, so a guard that refused only what is strictly older
/// would let the one request an attacker actually holds straight through.
#[test]
fn a_subscription_dated_the_same_second_as_the_record_cannot_replace_it() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    registry.accept(sub(200, 0xaa), 200).expect("first");

    assert_eq!(
        registry.accept(sub(200, 0xbb), 200),
        Err(AcceptError::Replayed)
    );
    assert_eq!(registry.lxmf_for(&[7u8; 32]).map(|a| a[15]), Some(0xaa));
}

#[test]
fn a_subscription_older_than_the_retention_window_is_refused() {
    let registry = Registry::ephemeral(Duration::from_secs(60));

    assert_eq!(
        registry.accept(sub(100, 0xaa), 200),
        Err(AcceptError::Lapsed)
    );
}

#[test]
fn a_subscription_delayed_but_still_inside_the_window_is_taken() {
    let registry = Registry::ephemeral(Duration::from_secs(60));

    assert!(registry.accept(sub(100, 0xaa), 159).is_ok());
}

#[test]
fn a_subscription_nobody_refreshed_stops_being_live() {
    let registry = Registry::ephemeral(Duration::from_secs(60));
    registry.accept(sub(100, 0xaa), 100).expect("accepted");

    assert_eq!(registry.live(150).len(), 1);
    assert!(registry.live(1000).is_empty());
}

#[test]
fn expiring_forgets_the_delivery_address_too() {
    let registry = Registry::ephemeral(Duration::from_secs(60));
    registry.accept(sub(100, 0xaa), 100).expect("accepted");

    assert_eq!(registry.expire(150), 0);
    assert_eq!(registry.expire(1000), 1);
    assert_eq!(registry.lxmf_for(&[7u8; 32]), None);
}

#[test]
fn a_routine_refresh_does_not_skip_events_not_yet_fetched() {
    let registry = Registry::ephemeral(Duration::from_secs(6000));
    registry.accept(sub(100, 0xaa), 100).expect("first");
    registry.mark_seen(&[7u8; 32], 1000);

    registry.accept(sub(1800, 0xaa), 1800).expect("refresh");

    assert_eq!(registry.last_seen(&[7u8; 32]), Some(1000));
}

#[test]
fn an_event_arriving_out_of_order_does_not_pull_the_mark_backwards() {
    let registry = Registry::ephemeral(Duration::from_secs(6000));
    registry.accept(sub(100, 0xaa), 100).expect("accepted");
    registry.mark_seen(&[7u8; 32], 1000);

    registry.mark_seen(&[7u8; 32], 500);

    assert_eq!(registry.last_seen(&[7u8; 32]), Some(1000));
}
