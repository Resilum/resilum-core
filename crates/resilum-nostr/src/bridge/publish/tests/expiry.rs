//! Rounds the relays never answered.

use std::time::Duration;

use super::*;
use crate::bridge::publish::round::DEADLINE;

/// The sweep reads each round's own clock: an event offered a moment later is
/// still owed the whole of its wait.
#[test]
fn a_round_out_of_time_is_settled_and_a_later_one_is_left_in_flight() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (first, first_request) = published("first");
    let (second, second_request) = published("second");
    let start = Instant::now();
    let later = start + Duration::from_millis(5);

    pending.offer(
        &bridge.publishing(),
        &first_request,
        Source::Recalled(PEER_A),
        start,
    );
    pending.offer(
        &bridge.publishing(),
        &second_request,
        Source::Recalled(PEER_B),
        later,
    );

    pending.expire(&bridge.publishing(), start + DEADLINE);

    let expired = bridge.acked_at(PEER_A);
    assert_eq!(expired.len(), 1, "its sender is told rather than left");
    assert_eq!(expired[0]["accepted"], 0);
    assert!(
        bridge.acked_at(PEER_B).is_empty(),
        "the later round has time left on its own clock"
    );

    pending.verdict(&bridge.publishing(), &second.id, verdict(true));
    assert_eq!(
        bridge.acked_at(PEER_B).len(),
        1,
        "and it is still in flight, so its relay's answer reaches it"
    );
    assert_eq!(bridge.broadcasts_of(&first.id), 1);
}
