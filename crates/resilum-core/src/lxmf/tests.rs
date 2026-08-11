use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::DeliveryMethod;
use leviculum_lxmf::router::{MessageState, RouterEvent};
use serde_json::{Value, json};

use super::poll::event_to_json;
use super::send::build_message;

#[test]
fn send_roundtrips_body_and_structured_custom_data() {
    let id = crate::identity::generate();
    let source_hash = crate::identity::lxmf_address(&id);
    let dest = "00112233445566778899aabbccddeeff";
    let content_b64 = BASE64.encode(b"hello");
    let req = json!({
        "dest": dest,
        "method": "direct",
        "content_b64": content_b64,
        "fields": {
            "custom_type": "rcb/1",
            "custom_data": { "n": 42, "ok": true, "items": [1, 2, 3] },
        },
    })
    .to_string();

    let msg = build_message(&req, &id, source_hash, 1.5).expect("build");
    assert_eq!(HEXLOWER.encode(&msg.destination_hash), dest);
    assert_eq!(msg.content, b"hello");
    assert_eq!(msg.method, DeliveryMethod::Direct);

    let out = event_to_json(&RouterEvent::MessageReceived(Box::new(msg))).expect("json");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["type"], "message");
    assert_eq!(v["source"], HEXLOWER.encode(&source_hash));
    assert_eq!(v["content_b64"], content_b64);
    assert_eq!(v["fields"]["custom_type"], "rcb/1");
    assert_eq!(v["fields"]["custom_data"]["n"], 42);
    assert_eq!(v["fields"]["custom_data"]["ok"], true);
    assert_eq!(v["fields"]["custom_data"]["items"], json!([1, 2, 3]));
}

/// The state strings are the app's contract, not an internal name: a UI keys
/// its delivery ticks off them, so a rename here is a silently broken client.
#[test]
fn every_message_state_has_its_app_facing_name() {
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
        let v: Value = serde_json::from_str(&event_to_json(&event).unwrap()).unwrap();
        assert_eq!(v["type"], "delivery");
        assert_eq!(v["state"], name);
        assert_eq!(v["message_id"], HEXLOWER.encode(&mid));
    }
}

#[test]
fn rejects_bad_method_and_dest() {
    let id = crate::identity::generate();
    let sh = crate::identity::lxmf_address(&id);
    let bad_method = r#"{"dest":"00112233445566778899aabbccddeeff","method":"bogus"}"#;
    let bad_dest = r#"{"dest":"xyz","method":"direct"}"#;
    assert!(build_message(bad_method, &id, sh, 0.0).is_err());
    assert!(build_message(bad_dest, &id, sh, 0.0).is_err());
}
