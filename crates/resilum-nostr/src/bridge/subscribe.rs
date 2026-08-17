//! A device asking this bridge to carry its direct messages.

use std::sync::Arc;

use serde_json::Value;

use super::state::{self, State};
use crate::event::{self, Event};
use crate::subscription;

pub(super) fn accept(state: &Arc<State>, data: &Value) {
    let event = match decode(data) {
        Ok(event) => event,
        Err(error) => return refused(&error),
    };
    let now = state::now();
    let sub = match subscription::from_event(&event, now) {
        Ok(sub) => sub,
        Err(error) => return refused(&error.to_string()),
    };
    // Between validating the request and writing it down: a bridge with an
    // allow list is a personal one, and this is the only thing that says so.
    if !state.cfg.may_use(&sub.pubkey) {
        return refused("this bridge does not carry that npub");
    }
    // No verification gate here: the signed `lxmf` tag already binds this
    // address to `sub.pubkey`, and a first subscription is the likeliest
    // moment this node has not yet recalled the sender's identity.
    let reissue = match state.registry.accept(sub, now) {
        Ok(reissue) => reissue,
        Err(error) => return refused(&error.to_string()),
    };
    // A refresh reuses its existing batch, so there is nothing to reissue.
    let Some(batch) = reissue else {
        tracing::info!("the bridge refreshed a subscription");
        return;
    };
    let relays = state.broadcast_batch(batch);
    tracing::info!(relays, %batch, "the bridge took a subscription");
}

fn decode(data: &Value) -> Result<Event, String> {
    let encoded = data.as_str().ok_or("custom_data is not a string")?;
    let raw = data_encoding::BASE64
        .decode(encoded.as_bytes())
        .map_err(|_| "custom_data is not valid base64")?;
    let json = String::from_utf8(raw).map_err(|_| "the request is not utf-8")?;
    event::parse(&json).map_err(|e| e.to_string())
}

fn refused(reason: &str) {
    tracing::warn!(reason, "a subscription request was refused");
}
