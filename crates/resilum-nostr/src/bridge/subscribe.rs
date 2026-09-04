//! A device asking this bridge to carry its direct messages.

use std::sync::Arc;

use serde_json::Value;

use super::deliver;
use super::state::{self, State};
use crate::config::NostrConfig;
use crate::event::{self, Event};
use crate::registry::Registry;
use crate::subscription::{self, Subscription};

pub(in crate::bridge) use outcome::{Outcome, Refusal};

mod outcome;

pub(super) fn accept(state: &Arc<State>, data: &Value, source: [u8; 16]) {
    deliver::subscription_answered(state, source, decide(state, data));
}

fn decide(state: &Arc<State>, data: &Value) -> Outcome {
    let event = match decode(data) {
        Ok(event) => event,
        Err(what) => return refused(Refusal::Malformed, &what),
    };
    let now = state::now();
    let sub = match subscription::from_event(&event, now) {
        Ok(sub) => sub,
        Err(error) => return refused(Refusal::from(&error), &error.to_string()),
    };
    if let Some((refusal, detail)) = refusal_for(&state.registry, &state.cfg, &sub, now) {
        return refused(refusal, &detail);
    }
    // No verification gate here: the signed `lxmf` tag already binds this
    // address to `sub.pubkey`, and a first subscription is the likeliest
    // moment this node has not yet recalled the sender's identity.
    let reissue = match state.registry.accept(sub, now) {
        Ok(reissue) => reissue,
        Err(error) => return refused(Refusal::from(&error), &error.to_string()),
    };
    // A refresh reuses its existing batch, so there is nothing to reissue.
    match reissue {
        None => tracing::info!("the bridge refreshed a subscription"),
        Some(batch) => {
            let relays = state.broadcast_batch(batch);
            tracing::info!(relays, %batch, "the bridge took a subscription");
        }
    }
    Outcome::Accepted {
        read_on: state.cfg.upstreams.clone(),
    }
}

fn refusal_for(
    registry: &Registry,
    cfg: &NostrConfig,
    sub: &Subscription,
    now: i64,
) -> Option<(Refusal, String)> {
    if let Err(error) = registry.ensure_fresh(sub, now) {
        return Some((Refusal::from(&error), error.to_string()));
    }
    if cfg.may_use(&sub.pubkey) {
        return None;
    }
    Some((
        Refusal::NotCarried,
        "this bridge does not carry that npub".to_owned(),
    ))
}

fn decode(data: &Value) -> Result<Event, String> {
    let encoded = data.as_str().ok_or("custom_data is not a string")?;
    let raw = data_encoding::BASE64
        .decode(encoded.as_bytes())
        .map_err(|_| "custom_data is not valid base64")?;
    let json = String::from_utf8(raw).map_err(|_| "the request is not utf-8")?;
    event::parse(&json).map_err(|e| e.to_string())
}

fn refused(refusal: Refusal, detail: &str) -> Outcome {
    tracing::warn!(detail, "a subscription request was refused");
    Outcome::Refused(refusal)
}

#[cfg(test)]
mod tests;
