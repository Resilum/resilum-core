//! Route a Nostr event from a relay back onto the mesh to its subscriber.

use data_encoding::HEXLOWER;
use serde_json::json;

use super::schema::SCHEMA_EVENT;
use crate::queue::Entry;

/// `content_b64` is omitted so a foreign LXMF client sees an empty message,
/// not ciphertext.
///
/// `destination` comes from `entry.lxmf`, not a registry lookup: a retry must
/// land where the event was addressed when it arrived.
///
/// `direct` is the only delivery method that yields the `delivered` state the
/// queue uses to drop an entry.
///
/// `timestamp` is pinned to `entry.queued_at` so every retry re-packs
/// byte-identically and a receiver collapses the repeat.
pub(super) fn send_request(entry: &Entry) -> String {
    let event_b64 = data_encoding::BASE64.encode(entry.event_json.as_bytes());

    let request = json!({
        "destination": HEXLOWER.encode(&entry.lxmf),
        "method": "direct",
        "timestamp": entry.queued_at as f64,
        "fields": {
            "custom_type": SCHEMA_EVENT,
            "custom_data": event_b64
        }
    });

    request.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::Direction;

    /// Feeds `send_request`'s output to `resilum_core::lxmf::send::build_message`,
    /// the actual consumer, instead of restating its field names: a rename on
    /// either side that broke delivery would otherwise need two agreeing,
    /// silent edits to escape notice.
    ///
    /// A foreign LXMF client that receives one of these must see an empty
    /// message, not a wall of ciphertext — so the body stays out of it.
    #[test]
    fn a_delivery_carries_the_event_in_fields_and_nothing_in_the_body() {
        let entry = Entry {
            direction: Direction::Inbound,
            subscriber: [7u8; 32],
            lxmf: [0x33; 16],
            event_id: [1u8; 32],
            event_json: r#"{"kind":1059}"#.into(),
            queued_at: 0,
        };

        let identity = resilum_core::identity::generate();
        let source_hash = resilum_core::identity::lxmf_address(&identity);
        let msg = resilum_core::lxmf::send::build_message(
            &send_request(&entry),
            &identity,
            source_hash,
            0.0,
        )
        .expect("the real consumer accepts what send_request produces");

        assert_eq!(msg.destination_hash, entry.lxmf);
        assert_eq!(format!("{:?}", msg.method), "Direct");
        assert!(msg.content.is_empty(), "nothing in the body");
        assert_eq!(
            msg.fields.len(),
            2,
            "custom_type and custom_data both reached the consumer"
        );
        // Spelled out rather than compared against SCHEMA_EVENT: the tag is
        // what a receiver dispatches on, so editing the constant has to break
        // something here rather than move silently with it.
        assert!(
            msg.fields
                .iter()
                .any(|(_, raw)| String::from_utf8_lossy(raw).contains("rsl.nostr/1")),
            "the schema tag on the wire is the one receivers dispatch on"
        );
        let event_b64 = data_encoding::BASE64.encode(entry.event_json.as_bytes());
        assert!(
            msg.fields
                .iter()
                .any(|(_, raw)| String::from_utf8_lossy(raw).contains(&event_b64)),
            "the event bytes reached the consumer unmodified"
        );
    }

    /// A retry calls `send_request` again for the same entry; the timestamp
    /// has to come from the entry, not the clock, or two attempts pack two
    /// different payloads and a receiver sees them as two different messages.
    #[test]
    fn the_timestamp_is_the_entrys_queued_at_and_stable_across_retries() {
        let entry = Entry {
            direction: Direction::Inbound,
            subscriber: [7u8; 32],
            lxmf: [0x33; 16],
            event_id: [1u8; 32],
            event_json: r#"{"kind":1059}"#.into(),
            queued_at: 1_723_000_000,
        };

        let first = send_request(&entry);
        let second = send_request(&entry);
        assert_eq!(first, second);

        let request: serde_json::Value = serde_json::from_str(&first).expect("valid json");
        assert_eq!(request["timestamp"], 1_723_000_000.0);
    }
}
