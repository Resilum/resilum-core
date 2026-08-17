//! What the queue is left holding once a round is over, and what a retry out
//! of it owes.

use super::*;

/// Two peers forwarding one event that no relay will take, far enough apart
/// that the first round is over before the second begins. The queue appends
/// what it is given, so nothing but the check against what it already holds
/// keeps this to one entry.
#[test]
fn an_event_already_held_is_not_queued_again_by_a_later_peer() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (event, request) = published("twice refused");

    offer(&bridge, &mut pending, &request, PEER_A);
    pending.verdict(&bridge.publishing(), &event.id, verdict(false));
    offer(&bridge, &mut pending, &request, PEER_B);
    pending.verdict(&bridge.publishing(), &event.id, verdict(false));

    assert_eq!(
        bridge.broadcasts_of(&event.id),
        2,
        "the rounds are separate, so this is not the shared-round case"
    );
    let queued = bridge.queue.due(state::now());
    assert_eq!(queued.len(), 1, "one event, one subscriber, one entry");
    assert_eq!(
        queued[0].lxmf, PEER_A,
        "held for the peer that offered first"
    );
}

/// A retry off the maintenance tick with nothing else in flight, so it opens
/// its own round. Its sender was answered when the event first arrived;
/// answering it again would tell a device the same failure at every retry for
/// the whole of the retention window.
#[test]
fn a_retry_opening_its_own_round_answers_nobody() {
    let bridge = held(1);
    let mut pending = Pending::default();
    let (event, _) = published("retried alone");

    let sent = pending.republish(&bridge.publishing(), &held_entry(&event), Instant::now());

    assert!(sent, "a relay was offered it, so the attempt is charged");
    assert_eq!(
        bridge.broadcasts_of(&event.id),
        1,
        "nothing was in flight, so this round is its own"
    );
    every_relay_answers(&bridge, &mut pending, &event, &[false]);
    assert!(
        bridge.acked_at(PEER_B).is_empty(),
        "the address it is held for hears nothing"
    );
}

/// A round nobody will ever return a verdict on. The answer is known the
/// moment it is asked for, and waiting out the deadline for it would leave
/// the sender hanging five seconds for news that was ready at once.
#[test]
fn a_sender_whose_bridge_holds_no_relay_is_told_so_without_waiting() {
    let bridge = held(0);
    let mut pending = Pending::default();
    let (_, request) = published("nowhere to go");

    offer(&bridge, &mut pending, &request, PEER_A);

    let acks = bridge.acked_at(PEER_A);
    assert_eq!(acks.len(), 1, "answered before any deadline is swept");
    assert_eq!(acks[0]["accepted"], 0);
    assert_eq!(acks[0]["reason"], "no relay is connected");
}

#[test]
fn a_retry_no_relay_is_connected_for_is_not_charged_an_attempt() {
    let bridge = held(0);
    let mut pending = Pending::default();
    let (event, _) = published("no relay to take it");

    let sent = pending.republish(&bridge.publishing(), &held_entry(&event), Instant::now());

    assert!(!sent, "nothing left the bridge");
}
