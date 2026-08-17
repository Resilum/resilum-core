use super::super::*;

/// The only branch that runs a nested typed deserialise, and the one every
/// message this bridge carries goes through.
#[test]
fn an_event_frame_hands_back_the_event_intact() {
    let event = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "hello");
    let body = serde_json::to_string(&event).expect("re-encodes");

    let incoming = parse_incoming(&format!(r#"["EVENT","b0",{body}]"#));

    match incoming {
        Some(Incoming::Event { event: carried }) => {
            assert_eq!(carried.id, event.id);
            assert!(carried.verify().is_ok(), "every signed field survived");
        }
        other => panic!("expected an event, got {other:?}"),
    }
}

#[test]
fn an_event_frame_without_a_subscription_id_is_refused() {
    let event = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "hello");
    let body = serde_json::to_string(&event).expect("re-encodes");

    assert!(parse_incoming(&format!(r#"["EVENT",{body}]"#)).is_none());
}

#[test]
fn a_relays_verdict_is_read_back() {
    let incoming = parse_incoming(r#"["OK","e0a4",false,"blocked: pow too low"]"#);

    match incoming {
        Some(Incoming::Verdict {
            event_id,
            accepted,
            message,
        }) => {
            assert_eq!(event_id, "e0a4");
            assert!(!accepted);
            assert_eq!(message, "blocked: pow too low");
        }
        other => panic!("expected an OK verdict, got {other:?}"),
    }
}

#[test]
fn a_refused_subscription_is_read_back_with_the_relays_reason() {
    let incoming = parse_incoming(r#"["CLOSED","17162c92","auth-required: we need it"]"#);

    match incoming {
        Some(Incoming::Closed {
            subscription,
            message,
        }) => {
            assert_eq!(subscription, "17162c92");
            assert_eq!(message, "auth-required: we need it");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_frame_we_do_not_model_is_ignored_rather_than_fatal() {
    assert!(parse_incoming(r#"["AUTH","challenge"]"#).is_none());
    assert!(parse_incoming("not json").is_none());
    assert!(parse_incoming("[]").is_none());
}

/// A modelled type sent with the wrong arity or field types must be ignored
/// the same way an unmodelled type is — not panic, and not parsed into a
/// verdict built from a field that was never there.
#[test]
fn a_malformed_frame_of_a_known_type_is_ignored_not_misread() {
    assert!(parse_incoming(r#"["OK","id-only"]"#).is_none());
    assert!(parse_incoming(r#"["EVENT"]"#).is_none());
    // Callers compare `event_id` against a queued entry's hex id.
    assert!(parse_incoming(r#"["OK", 1, true]"#).is_none());
}
