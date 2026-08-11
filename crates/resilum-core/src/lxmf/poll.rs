//! A `RouterEvent` as the JSON the FFI polls.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::Message;
use leviculum_lxmf::router::{MessageState, RouterEvent};
use serde_json::{Value, json};

pub fn event_to_json(event: &RouterEvent) -> Option<String> {
    let value = match event {
        RouterEvent::MessageReceived(message) => message_value(message),
        RouterEvent::MessageState { message_id, state } => {
            delivery_value(message_id, state_name(*state))
        }
        // The router reports these against a message id the app never learned:
        // an inbound duplicate or a bad signature is a message that, as far as
        // the app is concerned, did not arrive.
        _ => return None,
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

/// Exhaustive on purpose: an unnamed state is a message stuck in the UI with
/// no explanation, so a new one has to be answered for here.
fn state_name(state: MessageState) -> &'static str {
    match state {
        MessageState::Generating => "generating",
        MessageState::Outbound => "queued",
        MessageState::Sending => "sending",
        MessageState::Sent => "sent",
        // In a mailbox, not with the recipient — showing it as `delivered`
        // would claim an arrival that has not happened.
        MessageState::AwaitingCollection => "awaiting_collection",
        MessageState::Delivered => "delivered",
        MessageState::Rejected => "rejected",
        MessageState::Cancelled => "cancelled",
        MessageState::Failed => "failed",
    }
}

/// The `failed` update for a message the router refused outright, which it
/// reports through its return value rather than as an event.
pub(super) fn failed(message_id: &[u8; 32]) -> String {
    delivery_value(message_id, "failed").to_string()
}

fn delivery_value(message_id: &[u8; 32], state: &str) -> Value {
    json!({
        "type": "delivery",
        "message_id": HEXLOWER.encode(message_id),
        "state": state,
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
    })
}
