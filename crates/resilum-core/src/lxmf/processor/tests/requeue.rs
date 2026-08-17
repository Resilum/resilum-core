//! Moving a queued message onto another delivery method.

mod events;
mod queue;

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::DeliveryMethod;
use serde_json::{Value, json};

use super::harness::{Harness, STAMP_COST};
use super::node::known_peer;
use crate::lxmf::handle::Command;

/// An id no message in this harness can have: every real one is a hash.
const ABSENT: [u8; 32] = [9; 32];

fn submit_direct(harness: &mut Harness) -> [u8; 32] {
    let request = json!({
        "destination": HEXLOWER.encode(&known_peer(&mut harness.core)),
        "method": "direct",
        "content_b64": BASE64.encode(b"nobody answered"),
    })
    .to_string();
    let source_hash = crate::identity::lxmf_address(&harness.identity);
    let message = crate::lxmf::send::build_message(&request, &harness.identity, source_hash, 1.0)
        .expect("message");
    let message_id = message.message_id;
    harness
        .commands
        .send(Command::Send(Box::new(message)))
        .expect("submit");
    harness.pump();
    message_id
}

fn requeue(harness: &mut Harness, message_id: [u8; 32], method: DeliveryMethod) {
    harness
        .commands
        .send(Command::Requeue { message_id, method })
        .expect("requeue");
    harness.pump();
}

fn method_of(harness: &Harness, message_id: &[u8; 32]) -> Option<DeliveryMethod> {
    harness
        .ready
        .router
        .outbound()
        .get(message_id)
        .map(|entry| entry.message().method)
}

fn events(harness: &Harness) -> Vec<Value> {
    let mut events = Vec::new();
    while let Some(json) = harness.handle.next_event() {
        events.push(serde_json::from_str(&json).expect("event json"));
    }
    events
}

fn only_event(harness: &Harness, kind: &str) -> Value {
    let mut matching = events(harness);
    matching.retain(|event| event["type"] == kind);
    assert_eq!(matching.len(), 1, "expected one {kind}, got {matching:?}");
    matching.remove(0)
}
