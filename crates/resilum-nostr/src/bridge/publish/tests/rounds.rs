//! One event, several peers offering it, and one answer from the relays.

use super::*;

/// The case the shared round exists for: each peer is owed an answer at its
/// own address, and the relays are shown the event once.
#[test]
fn two_peers_offering_one_event_are_both_answered_and_the_relays_see_it_once() {
    let bridge = held(2);
    let mut pending = Pending::default();
    let (event, request) = published("shared");

    offer(&bridge, &mut pending, &request, PEER_A);
    offer(&bridge, &mut pending, &request, PEER_B);
    assert_eq!(
        bridge.broadcasts_of(&event.id),
        1,
        "offered once, not twice"
    );

    every_relay_answers(&bridge, &mut pending, &event, &[true, true]);

    for peer in [PEER_A, PEER_B] {
        let acks = bridge.acked_at(peer);
        assert_eq!(acks.len(), 1, "each peer hears once, at its own address");
        assert_eq!(acks[0]["accepted"], 2);
    }
}

/// An event one relay refused and another took is on the network, so both
/// senders hear that it went out.
#[test]
fn a_relay_refusing_beside_one_that_takes_it_still_tells_both_peers_it_went_out() {
    let bridge = held(2);
    let mut pending = Pending::default();
    let (event, request) = published("split");

    offer(&bridge, &mut pending, &request, PEER_A);
    offer(&bridge, &mut pending, &request, PEER_B);

    every_relay_answers(&bridge, &mut pending, &event, &[false, true]);

    for peer in [PEER_A, PEER_B] {
        let acks = bridge.acked_at(peer);
        assert_eq!(acks.len(), 1, "each peer hears once");
        assert_eq!(acks[0]["accepted"], 1, "a relay took it: {}", acks[0]);
    }
    assert!(
        bridge.queue.due(state::now()).is_empty(),
        "a relay is holding it, so nothing is owed a retry"
    );
}

#[test]
fn an_event_no_relay_took_is_held_once_and_for_the_peer_that_offered_it_first() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (event, request) = published("refused");

    offer(&bridge, &mut pending, &request, PEER_A);
    offer(&bridge, &mut pending, &request, PEER_B);

    every_relay_answers(&bridge, &mut pending, &event, &[false]);

    let queued = bridge.queue.due(state::now());
    assert_eq!(queued.len(), 1, "one event, one subscriber, one entry");
    assert_eq!(queued[0].lxmf, PEER_A);
    for peer in [PEER_A, PEER_B] {
        let acks = bridge.acked_at(peer);
        assert_eq!(acks[0]["accepted"], 0);
        assert_eq!(acks[0]["reason"], REFUSED);
    }
}

/// A queued event offered again while a peer is still waiting on it: the
/// relays have it, so nothing goes out, and its sender was answered when it
/// first arrived, so the waiting peer's claim is the only one.
#[test]
fn a_queued_event_offered_again_joins_the_round_without_re_sending_it() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (event, request) = published("again");
    offer(&bridge, &mut pending, &request, PEER_A);

    let sent = pending.republish(&bridge.publishing(), &held_entry(&event), Instant::now());

    assert!(!sent, "nothing went out, so no attempt is charged for it");
    assert_eq!(bridge.broadcasts_of(&event.id), 1, "the relays have it");
    every_relay_answers(&bridge, &mut pending, &event, &[true]);
    assert_eq!(bridge.acked_at(PEER_A).len(), 1);
    assert!(bridge.acked_at(PEER_B).is_empty(), "a retry owes nobody");
}

/// A peer whose acknowledgement was lost re-sends the event, and the round it
/// already joined is still open.
#[test]
fn a_peer_offering_one_event_twice_is_still_answered_once() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (event, request) = published("resent");

    offer(&bridge, &mut pending, &request, PEER_A);
    offer(&bridge, &mut pending, &request, PEER_A);

    every_relay_answers(&bridge, &mut pending, &event, &[true]);

    assert_eq!(
        bridge.acked_at(PEER_A).len(),
        1,
        "one round, one answer, however often it was asked for"
    );
}
