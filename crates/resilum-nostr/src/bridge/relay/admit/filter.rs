//! The terms an arriving event has to meet before it costs anything.
//!
//! These are the terms the `REQ` asked on, checked again on the way in:
//! signing is free, so an upstream that is hostile, compromised or merely
//! non-conforming would otherwise spend a subscriber's disk and airtime on
//! anything it liked.

use crate::config::NostrConfig;
use crate::event::{Event, decode_tag_hex};

/// The largest inbound event this bridge will carry.
///
/// NIP-44 refuses to encrypt more than 65535 bytes, so the seal inside a
/// NIP-17 gift wrap can be no larger; padded, MAC'd and base64-encoded that
/// reaches 87472 bytes, and the event around it adds an id, a pubkey, a
/// signature and a `p` tag — some 430 more. No conforming gift wrap passes
/// 86 KiB. The per-subscriber queue is a count of entries, so this is what
/// bounds it in bytes.
const MAX_EVENT_BYTES: usize = 96 * 1024;

/// `Err` carries the reason the caller logs.
pub(super) fn asked_for(
    cfg: &NostrConfig,
    subscriber: &[u8; 32],
    event: &Event,
    event_json: &str,
) -> Result<(), String> {
    if event_json.len() > MAX_EVENT_BYTES {
        return Err(format!(
            "{} bytes is past the {MAX_EVENT_BYTES}-byte ceiling",
            event_json.len()
        ));
    }
    if !cfg.inbound_kinds().contains(&event.kind) {
        return Err(format!(
            "kind {} is not one this bridge carries",
            event.kind
        ));
    }
    if !addressed_to(event, subscriber) {
        return Err("no p tag names this subscriber".into());
    }
    Ok(())
}

/// The subscribers an event names. A tag that is not 32 bytes of hex names
/// nobody.
pub(super) fn addressed(event: &Event) -> Vec<[u8; 32]> {
    event
        .tags
        .iter()
        .filter(|tag| tag.first().is_some_and(|name| name == "p"))
        .filter_map(|tag| tag.get(1))
        .filter_map(|value| decode_tag_hex::<32>(value).ok())
        .collect()
}

fn addressed_to(event: &Event, subscriber: &[u8; 32]) -> bool {
    addressed(event).contains(subscriber)
}
