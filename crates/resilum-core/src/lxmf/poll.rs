//! A `RouterEvent` as the JSON the FFI polls.
//!
//! Every match in this module and its children is exhaustive on purpose. These
//! tokens are the caller's contract, and a `_` arm would answer for a variant
//! upstream added after they were written by silently picking whichever label
//! it happened to land on. Failing to compile is the cheaper answer.

mod reason;
mod refusal;

use data_encoding::{BASE64, HEXLOWER};
use leviculum_core::DestinationHash;
use leviculum_lxmf::announce::DeliveryAnnounce;
use leviculum_lxmf::router::{MessageState, RouterEvent};
use leviculum_lxmf::{Message, Verification};
use refusal::failed_value;
pub(super) use refusal::{failed, requeue_refused};
use serde_json::{Value, json};

/// `heard_at` is when this node took the event off the router, in UNIX
/// seconds. Passed in rather than read here so the rendering stays a pure
/// function of its arguments.
pub fn event_to_json(event: &RouterEvent, heard_at: f64) -> Option<String> {
    let value = match event {
        RouterEvent::MessageReceived(message) => message_value(message),
        RouterEvent::PeerAnnounced {
            destination,
            announce,
        } => announce_value(destination, announce, heard_at),
        // The router's own give-up has no `RouterError` to translate — it is
        // reported as a bare state transition — so this is the one refusal
        // named here instead of by `reason::refusal`.
        RouterEvent::MessageState {
            message_id,
            state: MessageState::Failed,
        } => failed_value(message_id, reason::ATTEMPTS_EXHAUSTED),
        RouterEvent::MessageState { message_id, state } => {
            delivery_value(message_id, state_name(*state))
        }
        // Reported against a message id the caller never learned: an inbound
        // duplicate, a bad signature or an unpaid stamp is a message that, as
        // far as the caller is concerned, did not arrive.
        RouterEvent::Duplicate(_)
        | RouterEvent::InvalidSignature(_)
        | RouterEvent::InvalidStamp(_)
        | RouterEvent::InboundRejected { .. } => return None,
        // Router bookkeeping with no counterpart in the caller's model of a
        // message; the states it does model arrive as `MessageState`.
        RouterEvent::MessageQueued(_)
        | RouterEvent::DirectLinkEstablished { .. }
        | RouterEvent::ResourceBuildPending(_)
        | RouterEvent::PropagationSyncState(_)
        | RouterEvent::PropagationSyncComplete(_)
        | RouterEvent::PersistenceRequested => return None,
        // Work this crate owes the router, answered where it is raised.
        RouterEvent::StampPending(_)
        | RouterEvent::InboundStampPending(_)
        | RouterEvent::PropagationStampPending(_) => return None,
    };
    // A `json!` value fails to serialise only on a non-finite float, which
    // reaches here from a peer's message timestamp — an arrived message.
    match serde_json::to_string(&value) {
        Ok(json) => Some(json),
        Err(e) => {
            tracing::warn!(error = %e, "lxmf event could not be rendered");
            None
        }
    }
}

fn state_name(state: MessageState) -> &'static str {
    match state {
        MessageState::Generating => "generating",
        MessageState::Outbound => "queued",
        MessageState::Sending => "sending",
        MessageState::Sent => "sent",
        // On a propagation node, not with the recipient — showing it as
        // `delivered` would claim an arrival that has not happened.
        MessageState::AwaitingCollection => "awaiting_collection",
        MessageState::Delivered => "delivered",
        MessageState::Rejected => "rejected",
        MessageState::Cancelled => "cancelled",
        MessageState::Failed => "failed",
    }
}

fn delivery_value(message_id: &[u8; 32], state: &str) -> Value {
    json!({
        "type": "delivery",
        "message_id": HEXLOWER.encode(message_id),
        "state": state,
    })
}

/// A peer's `lxmf.delivery` announce, as the router decoded it. The router
/// filters the announce stream by the delivery name hash and parses the app
/// data for its own stamp-cost cache, so neither is redone here.
///
/// The timestamp is when this node heard the announce, not when the peer sent
/// it: an announce carries no trustworthy emission time, and the hop it came
/// over may have held it.
fn announce_value(
    destination: &DestinationHash,
    announce: &DeliveryAnnounce,
    heard_at: f64,
) -> Value {
    json!({
        "type": "announce",
        "source": HEXLOWER.encode(destination.as_bytes()),
        // Always present, empty when the peer announced no name: an absent key
        // and an anonymous peer would be indistinguishable to the caller.
        "display_name": announce.display_name().unwrap_or_default(),
        "timestamp": heard_at,
    })
}

fn message_value(message: &Message) -> Value {
    let (custom_type, custom_data) = super::decode_fields(&message.fields);
    json!({
        "type": "message",
        "source": HEXLOWER.encode(&message.source_hash),
        "message_id": HEXLOWER.encode(&message.message_id),
        "timestamp": message.timestamp,
        "title_b64": BASE64.encode(&message.title),
        "content_b64": BASE64.encode(&message.content),
        "fields": { "custom_type": custom_type, "custom_data": custom_data },
        "verification": verification_name(message.verification),
    })
}

fn verification_name(verification: Verification) -> &'static str {
    match verification {
        Verification::Valid => "valid",
        Verification::Unverified => "unverified",
        Verification::Invalid => "invalid",
    }
}

#[cfg(test)]
mod tests;
