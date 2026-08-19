//! One polled LXMF event, read once into the thing the loop does with it.

use serde_json::Value;

use super::publish::Source;
use super::schema::{SCHEMA_EVENT, SCHEMA_SUBSCRIBE};
use crate::event::decode_hex;

pub(super) enum Polled {
    Subscribe {
        data: Value,
        source: [u8; 16],
    },
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
        Some(SCHEMA_SUBSCRIBE) => match address(event["source"].as_str()) {
            Some(source) => Polled::Subscribe {
                data: event["fields"]["custom_data"].clone(),
                source,
            },
            None => {
                tracing::warn!("a subscription named no sender, so nothing can be sent back");
                Polled::Ignored
            }
        },
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
mod tests;
