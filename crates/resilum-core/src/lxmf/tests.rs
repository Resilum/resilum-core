use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::router::{MessageState, RouterEvent};
use leviculum_lxmf::{DeliveryMethod, Verification};
use serde_json::{Value, json};

use super::handle::{MAX_QUEUED_EVENTS, channel};
use super::inbox::{Inbox, MAX_HELD};
use super::poll::event_to_json;
use super::send::build_message;

#[test]
fn send_roundtrips_body_and_structured_custom_data() {
    let id = crate::identity::generate();
    let source_hash = crate::identity::lxmf_address(&id);
    let destination = "00112233445566778899aabbccddeeff";
    let content_b64 = BASE64.encode(b"hello");
    let req = json!({
        "destination": destination,
        "method": "direct",
        "content_b64": content_b64,
        "fields": {
            "custom_type": "rsl.rcb/1",
            "custom_data": { "n": 42, "ok": true, "items": [1, 2, 3] },
        },
    })
    .to_string();

    let msg = build_message(&req, &id, source_hash, 1.5).expect("build");
    assert_eq!(HEXLOWER.encode(&msg.destination_hash), destination);
    assert_eq!(msg.content, b"hello");
    assert_eq!(msg.method, DeliveryMethod::Direct);

    let out = event_to_json(&RouterEvent::MessageReceived(Box::new(msg)), 0.0).expect("json");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["type"], "message");
    assert_eq!(v["source"], HEXLOWER.encode(&source_hash));
    assert_eq!(v["content_b64"], content_b64);
    assert_eq!(v["fields"]["custom_type"], "rsl.rcb/1");
    assert_eq!(v["fields"]["custom_data"]["n"], 42);
    assert_eq!(v["fields"]["custom_data"]["ok"], true);
    assert_eq!(v["fields"]["custom_data"]["items"], json!([1, 2, 3]));
}

/// The state strings are the caller's contract, not an internal name: a
/// rename here is a silently broken consumer.
#[test]
fn every_message_state_has_its_public_name() {
    let mid = [7u8; 32];
    for (state, name) in [
        (MessageState::Generating, "generating"),
        (MessageState::Outbound, "queued"),
        (MessageState::Sending, "sending"),
        (MessageState::Sent, "sent"),
        (MessageState::AwaitingCollection, "awaiting_collection"),
        (MessageState::Delivered, "delivered"),
        (MessageState::Rejected, "rejected"),
        (MessageState::Cancelled, "cancelled"),
        (MessageState::Failed, "failed"),
    ] {
        let event = RouterEvent::MessageState {
            message_id: mid,
            state,
        };
        let v: Value = serde_json::from_str(&event_to_json(&event, 0.0).unwrap()).unwrap();
        assert_eq!(v["type"], "delivery");
        assert_eq!(v["state"], name);
        assert_eq!(v["message_id"], HEXLOWER.encode(&mid));
    }
}

/// The verification strings are the caller's and the Nostr bridge's contract:
/// `unverified` is the difference between a proven sender and a claimed one,
/// so a rename here silently turns that distinction back into a bare address.
#[test]
fn every_verification_state_has_its_public_name() {
    let id = crate::identity::generate();
    let source_hash = crate::identity::lxmf_address(&id);
    let req =
        json!({"destination": "00112233445566778899aabbccddeeff", "method": "direct"}).to_string();
    let mut msg = build_message(&req, &id, source_hash, 0.0).expect("build");

    for (verification, name) in [
        (Verification::Valid, "valid"),
        (Verification::Unverified, "unverified"),
        (Verification::Invalid, "invalid"),
    ] {
        msg.verification = verification;
        let out =
            event_to_json(&RouterEvent::MessageReceived(Box::new(msg.clone())), 0.0).expect("json");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["verification"], name);
    }
}

/// One of these means the sender has to repeat what it sent and the other
/// does not, so an unlabelled overflow is a lie either way.
#[test]
fn a_full_inbox_and_a_full_event_queue_are_told_apart() {
    let inbox = Arc::new(Inbox::ephemeral());
    let (handle, _commands, mut sink) =
        channel("aa".into(), Arc::new(AtomicBool::new(true)), inbox.clone());

    for i in 0..MAX_HELD + 2 {
        inbox.push(format!("{{\"type\":\"message\",\"n\":{i}}}"));
    }
    assert_eq!(drain_to_overflow(&handle)["kind"], "messages");

    for i in 0..=MAX_QUEUED_EVENTS {
        sink.push(format!("{{\"type\":\"delivery\",\"n\":{i}}}"));
    }
    while handle.next_event().is_some() {}
    sink.push(r#"{"type":"delivery"}"#.into());
    assert_eq!(drain_to_overflow(&handle)["kind"], "delivery");
}

fn drain_to_overflow(handle: &super::LxmfHandle) -> Value {
    std::iter::from_fn(|| handle.next_event())
        .map(|json| serde_json::from_str::<Value>(&json).expect("an event is json"))
        .find(|v| v["type"] == "overflow")
        .expect("an overflow is still queued")
}

/// Same id despite a changed fallback proves the request pins the payload;
/// the unpinned pair still diverging rules out coincidence.
#[test]
fn an_explicit_timestamp_pins_the_message_id_across_a_retry() {
    let id = crate::identity::generate();
    let sh = crate::identity::lxmf_address(&id);
    let destination = "00112233445566778899aabbccddeeff";
    let pinned =
        json!({"destination": destination, "method": "direct", "timestamp": 1.7e9}).to_string();
    let unpinned = json!({"destination": destination, "method": "direct"}).to_string();
    let id_of = |j: &str, t: f64| build_message(j, &id, sh, t).unwrap().message_id;
    assert_eq!(id_of(&pinned, 1.0), id_of(&pinned, 2.0));
    assert_ne!(id_of(&unpinned, 1.0), id_of(&unpinned, 2.0));
}

#[test]
fn rejects_bad_method_and_destination() {
    let id = crate::identity::generate();
    let sh = crate::identity::lxmf_address(&id);
    let bad_method = r#"{"destination":"00112233445566778899aabbccddeeff","method":"bogus"}"#;
    let bad_destination = r#"{"destination":"xyz","method":"direct"}"#;
    assert!(build_message(bad_method, &id, sh, 0.0).is_err());
    assert!(build_message(bad_destination, &id, sh, 0.0).is_err());
}
