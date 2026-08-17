//! Telling a sender its event went nowhere.
//!
//! A sender that hears nothing cannot tell a refusal from an event still on
//! its way, which is the failure `rsl.nostr.ack/1` exists to prevent — a
//! refusal has to be answered as surely as a rejection from a relay does.

use serde_json::Value;

use super::Publishing;
use crate::bridge::from_mesh::Verdicts;
use crate::event;

pub(super) fn answer(bridge: &Publishing<'_>, source: [u8; 16], data: &Value, reason: &str) {
    let Some((event_id, verdicts)) = ack_for(data, reason) else {
        tracing::warn!("the refused request named no event, so its sender cannot be told");
        return;
    };
    (bridge.ack)(source, &event_id, &verdicts);
}

/// The id comes back out of the request the sender made, not off a verified
/// event: not being able to trust the event is one of the reasons to be
/// here, and the sender is waiting on the id it sent.
fn ack_for(data: &Value, reason: &str) -> Option<(String, Verdicts)> {
    let raw = data_encoding::BASE64
        .decode(data.as_str()?.as_bytes())
        .ok()?;
    let json = String::from_utf8(raw).ok()?;
    let event_id = event::parse(&json).ok()?.id;
    let verdicts = Verdicts {
        accepted: 0,
        rejected: 0,
        reason: reason.to_owned(),
    };
    Some((event_id, verdicts))
}

#[cfg(test)]
mod tests;
