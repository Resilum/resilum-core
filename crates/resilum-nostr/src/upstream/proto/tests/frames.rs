use super::super::*;

/// Compare parsed values, not strings: JSON object key order is not part of
/// the protocol, and pinning it in a test would force a dependency feature on
/// the whole workspace to satisfy an assertion.
fn parsed(frame: &str) -> Value {
    serde_json::from_str(frame).expect("a frame we built is valid json")
}

fn filter(subscriber_pubkey_hex: &str, since: Option<i64>) -> Filter {
    Filter {
        kinds: vec![1059, 4],
        subscriber_pubkey_hex: subscriber_pubkey_hex.into(),
        since,
    }
}

#[test]
fn a_request_names_its_subscriber_and_resumes_where_it_stopped() {
    let frame = request_frame("aabb", &[filter("17162c92", Some(500))]);

    assert_eq!(
        parsed(&frame),
        json!([
            "REQ",
            "aabb",
            {"kinds": [1059, 4], "#p": ["17162c92"], "since": 500}
        ])
    );
}

#[test]
fn a_first_request_asks_for_everything_stored() {
    let frame = request_frame("aabb", &[filter("17162c92", None)]);

    assert_eq!(
        parsed(&frame),
        json!(["REQ", "aabb", {"kinds": [1059, 4], "#p": ["17162c92"]}])
    );
}

#[test]
fn a_batched_request_keeps_each_subscribers_own_resume_point() {
    let frame = request_frame(
        "b0",
        &[
            filter("17162c92", Some(500)),
            filter("aa01", None),
            filter("bb02", Some(900)),
        ],
    );

    assert_eq!(
        parsed(&frame),
        json!([
            "REQ",
            "b0",
            {"kinds": [1059, 4], "#p": ["17162c92"], "since": 500},
            {"kinds": [1059, 4], "#p": ["aa01"]},
            {"kinds": [1059, 4], "#p": ["bb02"], "since": 900}
        ])
    );
}

/// A relay echoes nothing back about an event it accepted, so an event
/// mangled on its way into a publish frame is a message that silently never
/// arrives.
#[test]
fn an_event_offered_to_a_relay_goes_out_whole() {
    let event = crate::signed::gift_wrap([9u8; 32], 1_786_733_083, "round trip");

    let frame = publish_frame(&event).expect("a well-formed event frames");

    let Value::Array(fields) = parsed(&frame) else {
        panic!("a relay frame is a json array");
    };
    assert_eq!(fields.first().and_then(Value::as_str), Some("EVENT"));
    let back: Event =
        serde_json::from_value(fields.into_iter().nth(1).expect("the event")).expect("an event");
    assert_eq!(back.id, event.id);
    assert!(back.verify().is_ok(), "every signed field survived");
}
