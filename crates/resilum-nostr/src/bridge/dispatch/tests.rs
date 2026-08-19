use super::*;

fn message(custom_type: &str) -> String {
    serde_json::json!({
        "type": "message",
        "source": "00112233445566778899aabbccddeeff",
        "message_id": "ab",
        "fields": { "custom_type": custom_type, "custom_data": "Zm9v" }
    })
    .to_string()
}

/// The two schemas differ only by a string; routing a subscription into
/// the publish path would offer someone's subscription to every relay.
#[test]
fn each_schema_classifies_as_its_own_variant() {
    assert!(matches!(
        classify(&message(SCHEMA_SUBSCRIBE)),
        Polled::Subscribe { .. }
    ));
    assert!(matches!(
        classify(&message(SCHEMA_EVENT)),
        Polled::Publish { .. }
    ));
    assert!(matches!(classify(&message("rsl.rcb/1")), Polled::Ignored));
}

/// Reading any state but `delivered` as proof would discard an event that
/// never reached the device.
#[test]
fn only_a_delivered_state_classifies_as_polled_delivered() {
    for state in ["sent", "queued", "awaiting_collection", "failed"] {
        let json =
            serde_json::json!({"type": "delivery", "message_id": "ab", "state": state}).to_string();
        assert!(!matches!(classify(&json), Polled::Delivered(_)), "{state}");
    }
    let json = serde_json::json!({"type": "delivery", "message_id": "ab", "state": "delivered"})
        .to_string();
    assert!(matches!(classify(&json), Polled::Delivered(_)));
}

/// A publish claiming an address this node has not recalled must not read
/// as a recalled one: on a bridge with an allow list that is the whole of
/// the gate.
#[test]
fn only_a_valid_verification_recalls_the_sender() {
    for claim in [Some("unknown"), Some("invalid"), None] {
        assert!(
            matches!(sender([1u8; 16], claim), Source::Claimed(_)),
            "{claim:?}"
        );
    }
    assert!(matches!(
        sender([1u8; 16], Some("valid")),
        Source::Recalled(_)
    ));
}

#[test]
fn an_announce_classifies_as_the_address_that_sent_it() {
    let json = serde_json::json!({
        "type": "announce",
        "source": "00112233445566778899aabbccddeeff"
    })
    .to_string();

    assert!(matches!(
        classify(&json),
        Polled::Reachable(address) if address == [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ]
    ));
}
