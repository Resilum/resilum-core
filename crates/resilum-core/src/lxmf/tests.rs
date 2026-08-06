use super::poll::event_to_json;
use super::send::build_message;
use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::{DeliveryMethod, LxmfNodeEvent};
use serde_json::{Value, json};

#[test]
fn send_roundtrips_body_and_structured_custom_data() {
    let id = crate::identity::generate();
    let source_hash = *id.hash();
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

    let out = event_to_json(&LxmfNodeEvent::MessageReceived(msg)).expect("json");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["type"], "message");
    assert_eq!(v["source"], HEXLOWER.encode(&source_hash));
    assert_eq!(v["content_b64"], content_b64);
    assert_eq!(v["fields"]["custom_type"], "rcb/1");
    assert_eq!(v["fields"]["custom_data"]["n"], 42);
    assert_eq!(v["fields"]["custom_data"]["ok"], true);
    assert_eq!(v["fields"]["custom_data"]["items"], json!([1, 2, 3]));
}

#[test]
fn delivery_events_map_to_states() {
    let mid = [7u8; 32];
    for (event, state) in [
        (LxmfNodeEvent::Delivered { message_id: mid }, "delivered"),
        (
            LxmfNodeEvent::DeliveryFailed {
                message_id: mid,
                reason: leviculum_lxmf::DeliveryFailure::DirectPacketTimeout,
            },
            "failed",
        ),
    ] {
        let v: Value = serde_json::from_str(&event_to_json(&event).unwrap()).unwrap();
        assert_eq!(v["type"], "delivery");
        assert_eq!(v["state"], state);
        assert_eq!(v["message_id"], HEXLOWER.encode(&mid));
    }
}

#[test]
fn rejects_bad_method_and_dest() {
    let id = crate::identity::generate();
    let sh = *id.hash();
    let bad_method = r#"{"dest":"00112233445566778899aabbccddeeff","method":"bogus"}"#;
    let bad_dest = r#"{"dest":"xyz","method":"direct"}"#;
    assert!(build_message(bad_method, &id, sh, 0.0).is_err());
    assert!(build_message(bad_dest, &id, sh, 0.0).is_err());
}
