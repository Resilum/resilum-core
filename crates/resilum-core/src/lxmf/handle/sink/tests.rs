use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};

use super::EventSink;
use crate::lxmf::handle::{LxmfHandle, channel};
use crate::lxmf::inbox::Inbox;

const ALICE: [u8; 16] = [0xaa; 16];
const BOB: [u8; 16] = [0xbb; 16];

fn pair() -> (LxmfHandle, EventSink) {
    let inbox = Arc::new(Inbox::ephemeral());
    let (handle, _commands, sink) = channel("aa".into(), Arc::new(AtomicBool::new(true)), inbox);
    (handle, sink)
}

fn announce(display_name: &str) -> String {
    json!({"type": "announce", "display_name": display_name}).to_string()
}

fn drain(handle: &LxmfHandle) -> Vec<Value> {
    std::iter::from_fn(|| handle.next_event())
        .map(|json| serde_json::from_str(&json).expect("an event is json"))
        .collect()
}

#[test]
fn a_newer_announce_replaces_the_pending_one_from_that_peer() {
    let (handle, mut sink) = pair();

    sink.push_announce(ALICE, announce("old"));
    sink.push_announce(ALICE, announce("new"));

    // The count, not the drained events: a second entry pointing at the same
    // superseded announce reads back as one event while costing two slots.
    assert_eq!(handle.queued_events(), 1, "one place in the queue per peer");
    let events = drain(&handle);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["display_name"], "new");
}

#[test]
fn announces_from_different_peers_each_keep_their_place() {
    let (handle, mut sink) = pair();

    sink.push_announce(ALICE, announce("alice"));
    sink.push_announce(BOB, announce("bob"));

    let events = drain(&handle);
    let names: Vec<&Value> = events.iter().map(|e| &e["display_name"]).collect();
    assert_eq!(names, ["alice", "bob"]);
}

/// Coalescing must reach nothing but the announce it replaces: the delivery
/// update queued between the two is the verdict the caller is waiting on.
#[test]
fn a_delivery_update_between_two_announces_keeps_its_place() {
    let (handle, mut sink) = pair();

    sink.push_announce(ALICE, announce("old"));
    sink.push(json!({"type": "delivery", "state": "sent"}).to_string());
    sink.push_announce(ALICE, announce("new"));

    assert_eq!(
        handle.queued_events(),
        2,
        "the announce slot plus the update"
    );
    let events = drain(&handle);
    assert_eq!(events[0]["display_name"], "new");
    assert_eq!(events[1]["type"], "delivery");
}

/// Once an announce has been read the peer holds no place, so the next one
/// has to take a fresh one — otherwise a peer is heard from exactly once.
#[test]
fn a_peer_announces_again_after_the_first_has_been_read() {
    let (handle, mut sink) = pair();

    sink.push_announce(ALICE, announce("first"));
    assert!(handle.next_event().is_some(), "the first announce");
    sink.push_announce(ALICE, announce("second"));

    let events = drain(&handle);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["display_name"], "second");
}
