//! Something the router would not do, as the caller reads it.

use data_encoding::HEXLOWER;
use leviculum_lxmf::router::{MessageState, RouterError};
use serde_json::{Value, json};

use super::reason::{self, Refusal};

/// The `failed` update for a message the router refused outright, which it
/// reports through its return value rather than as an event.
pub(in crate::lxmf) fn failed(message_id: &[u8; 32], error: &RouterError) -> String {
    failed_value(message_id, reason::refusal(error)).to_string()
}

/// Why a [`crate::lxmf::LxmfHandle::requeue_with_method`] did not happen.
///
/// Deliberately not a `delivery` event: a refused re-queue leaves the message
/// exactly where it was, and for the common refusal — an id the router no
/// longer holds — that place is a terminal state the caller was already told
/// about. A `failed` here would overwrite a `delivered` with a lie.
pub(in crate::lxmf) fn requeue_refused(message_id: &[u8; 32], error: &RouterError) -> String {
    let refusal = reason::refusal(error);
    json!({
        "type": "requeue_refused",
        "message_id": HEXLOWER.encode(message_id),
        "reason": refusal.reason,
        "retry": refusal.retry.token(),
    })
    .to_string()
}

/// `reason`/`retry` are additive to a plain delivery update and only ever
/// present on `failed`, so they get their own constructor rather than an
/// `Option` threaded through the shared one.
pub(super) fn failed_value(message_id: &[u8; 32], refusal: Refusal) -> Value {
    json!({
        "type": "delivery",
        "message_id": HEXLOWER.encode(message_id),
        "state": super::state_name(MessageState::Failed),
        "reason": refusal.reason,
        "retry": refusal.retry.token(),
    })
}
