//! A mesh publish request read down to the round it opens — or refused, and
//! its sender told so.

use std::sync::Arc;

use serde_json::Value;

use super::{Publishing, Source, admit, refusal};
use crate::bridge::from_mesh;
use crate::bridge::tie::Tie;
use crate::event;
use crate::queue::Entry;
use crate::upstream::proto;

pub(super) struct Offered {
    pub(super) tie: Tie,
    pub(super) event_json: Arc<str>,
    pub(super) frame: String,
}

pub(super) fn offered(bridge: &Publishing<'_>, data: &Value, source: Source) -> Option<Offered> {
    // Before the signature check: an address this bridge does not serve
    // should not cost it an elliptic curve operation per message.
    if let Err(reason) = admit::may_publish(bridge.cfg, bridge.registry, source) {
        return refused(bridge, source, data, &reason);
    }
    let publish = match from_mesh::accept_publish(data, bridge.cfg) {
        Ok(publish) => publish,
        Err(reason) => return refused(bridge, source, data, &reason),
    };
    let (Some(event_id), Some(subscriber)) =
        (publish.event.id_bytes(), publish.event.pubkey_bytes())
    else {
        let reason = "the event's id or signing key is not 32 bytes of hex";
        return refused(bridge, source, data, reason);
    };
    let Ok(event_json) = serde_json::to_string(&publish.event) else {
        return refused(bridge, source, data, "the event could not be re-encoded");
    };
    Some(Offered {
        tie: Tie {
            event_id,
            subscriber,
        },
        event_json: event_json.into(),
        frame: publish.frame,
    })
}

fn refused(bridge: &Publishing<'_>, source: Source, data: &Value, reason: &str) -> Option<Offered> {
    tracing::warn!(reason, "a publish request was refused");
    refusal::answer(bridge, source.address(), data, reason);
    None
}

/// A queued event framed for the relays again. `None` when no attempt ever
/// could: such an entry is not owed a retry until retention, so it goes now.
pub(super) fn reframed(bridge: &Publishing<'_>, entry: &Entry) -> Option<String> {
    let tie = Tie::from(entry);
    let Ok(event) = event::parse(&entry.event_json) else {
        return dropped(bridge, tie, "a queued event is not an event any more");
    };
    let Some(frame) = proto::publish_frame(&event) else {
        return dropped(bridge, tie, "a queued event cannot be framed for a relay");
    };
    Some(frame)
}

fn dropped(bridge: &Publishing<'_>, tie: Tie, why: &'static str) -> Option<String> {
    tracing::warn!(why, "a queued event was dropped");
    bridge.queue.resolve(&tie.event_id, &tie.subscriber);
    None
}
