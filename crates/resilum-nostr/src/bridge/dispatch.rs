//! One polled LXMF event, read once into the thing the loop does with it.

use serde_json::Value;

use super::publish::Source;
use super::schema::{SCHEMA_EVENT, SCHEMA_SUBSCRIBE};
use crate::event::decode_hex;

pub(super) enum Polled {
    Subscribe(Value),
    Publish {
        data: Value,
        source: Source,
    },
    Delivered(String),
    /// Terminal, but not proof of anything: the entry stays owed.
    NotDelivered(String),
    Reachable([u8; 16]),
    Ignored,
}

pub(super) fn classify(json: &str) -> Polled {
    let Ok(event) = serde_json::from_str::<Value>(json) else {
        tracing::warn!("an lxmf event was not json");
        return Polled::Ignored;
    };
    match event["type"].as_str() {
        Some("message") => message(&event),
        Some("delivery") => delivery(&event),
        Some("announce") => match address(event["source"].as_str()) {
            Some(source) => Polled::Reachable(source),
            None => Polled::Ignored,
        },
        Some("overflow") => {
            tracing::warn!(
                kind = event["kind"].as_str().unwrap_or("?"),
                dropped = event["dropped"].as_u64().unwrap_or_default(),
                "lxmf events were dropped before the bridge read them"
            );
            Polled::Ignored
        }
        _ => Polled::Ignored,
    }
}

fn message(event: &Value) -> Polled {
    match event["fields"]["custom_type"].as_str() {
        Some(SCHEMA_SUBSCRIBE) => Polled::Subscribe(event["fields"]["custom_data"].clone()),
        Some(SCHEMA_EVENT) => match address(event["source"].as_str()) {
            // A gift wrap runs to tens of kilobytes of base64, so it is
            // cloned only where something will read it.
            Some(source) => Polled::Publish {
                data: event["fields"]["custom_data"].clone(),
                source: sender(source, event["verification"].as_str()),
            },
            None => {
                tracing::warn!("a publish request named no sender, so nothing can be sent back");
                Polled::Ignored
            }
        },
        _ => Polled::Ignored,
    }
}

/// Anything but `"valid"` counts as unrecalled, missing fields included: a
/// field the reader does not recognise is not proof of anything.
fn sender(address: [u8; 16], verification: Option<&str>) -> Source {
    match verification {
        Some("valid") => Source::Recalled(address),
        _ => Source::Claimed(address),
    }
}

fn delivery(event: &Value) -> Polled {
    let Some(id) = event["message_id"].as_str() else {
        return Polled::Ignored;
    };
    match event["state"].as_str() {
        Some("delivered") => Polled::Delivered(id.to_owned()),
        Some("rejected" | "cancelled" | "failed") => Polled::NotDelivered(id.to_owned()),
        _ => Polled::Ignored,
    }
}

fn address(hex: Option<&str>) -> Option<[u8; 16]> {
    decode_hex::<16>(hex?).ok()
}

#[cfg(test)]
mod tests {
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
            Polled::Subscribe(_)
        ));
        assert!(matches!(
            classify(&message(SCHEMA_EVENT)),
            Polled::Publish { .. }
        ));
        assert!(matches!(classify(&message("rcb/1")), Polled::Ignored));
    }

    /// Reading any state but `delivered` as proof would discard an event that
    /// never reached the device.
    #[test]
    fn only_a_delivered_state_classifies_as_polled_delivered() {
        for state in ["sent", "queued", "awaiting_collection", "failed"] {
            let json = serde_json::json!({"type": "delivery", "message_id": "ab", "state": state})
                .to_string();
            assert!(!matches!(classify(&json), Polled::Delivered(_)), "{state}");
        }
        let json =
            serde_json::json!({"type": "delivery", "message_id": "ab", "state": "delivered"})
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
}
