//! Whether a mesh-signed event is one this bridge will offer to the relays.

mod ack;

use serde_json::Value;

use crate::config::NostrConfig;
use crate::event::{self, Event};
use crate::upstream::proto;

pub(in crate::bridge) use ack::{Verdicts, ack_json};

/// An event accepted for publishing, already framed for the relay wire.
#[derive(Debug)]
pub(in crate::bridge) struct Publish {
    pub(in crate::bridge) event: Event,
    pub(in crate::bridge) frame: String,
}

/// Verification runs before the kind check: an unsigned or tampered event
/// must never reach a relay, whatever kind it claims.
///
/// Says nothing about who may publish: the caller gates on the mesh address
/// the request arrived from instead.
///
/// `Err` carries the reason, which reaches the sender in its ack.
pub(in crate::bridge) fn accept_publish(
    custom_data: &Value,
    cfg: &NostrConfig,
) -> Result<Publish, String> {
    let encoded = custom_data.as_str().ok_or("custom_data is not a string")?;
    let raw = data_encoding::BASE64
        .decode(encoded.as_bytes())
        .map_err(|_| "custom_data is not valid base64")?;
    let json_text = String::from_utf8(raw).map_err(|_| "decoded event is not utf-8")?;

    let event = event::parse(&json_text).map_err(|e| e.to_string())?;
    event.verify().map_err(|e| e.to_string())?;

    if !cfg.publish_kinds.contains(&event.kind) {
        return Err(format!(
            "kind {} is not one this bridge carries",
            event.kind
        ));
    }

    let frame = proto::publish_frame(&event).ok_or("event could not be framed for the relay")?;
    Ok(Publish { event, frame })
}

#[cfg(test)]
mod tests;
