use std::ffi::CString;
use std::os::raw::c_char;

use crate::node::ResilumNode;
use crate::{clear_error, guard, set_error};

/// Next LXMF event as JSON, or null. One of:
/// ```json
/// { "type":"message", "source":"<hex>", "message_id":"<hex>",
///   "timestamp": 0.0, "title_b64":"...", "content_b64":"...",
///   "fields": { "custom_type":"...", "custom_data": <value> },
///   "verification":"valid" | "unverified" | "invalid" }
/// { "type":"delivery", "message_id":"<hex>", "state":"generating" | "queued" |
///   "sending" | "sent" | "awaiting_collection" | "delivered" | "rejected" |
///   "cancelled" | "failed" }
/// { "type":"delivery", "message_id":"<hex>", "state":"failed",
///   "reason":"<token>", "retry":"resubmit" | "requeue" | "never" }
/// { "type":"announce", "source":"<32-hex lxmf address>",
///   "display_name":"<may be empty>", "timestamp": 0.0 }
/// { "type":"overflow", "kind":"messages" | "delivery", "dropped": 0 }
/// { "type":"requeue_refused", "message_id":"<hex>", "reason":"<token>",
///   "retry":"resubmit" | "requeue" | "never" }
/// ```
///
/// **Null is not only "queue empty".** It also means a null node, an event the
/// node could not render as a C string, or a caught panic. This call clears the
/// thread-local error slot on entry, so after it returns null a null
/// `resilum_last_error()` means the queue was empty and a non-null one is this
/// call's own failure — never a stale message from an earlier call. A poll loop
/// that treats every null as idle otherwise spins forever on a hard failure.
///
/// `delivery` tracks a message this node sent, keyed by the id
/// `resilum_lxmf_send` returned. `overflow` says how many events were discarded
/// because a queue filled up, which happens only if the caller stops polling
/// while the mesh keeps delivering: `"kind":"messages"` means received messages
/// are gone and their senders have to repeat them, `"delivery"` only that some
/// delivery states were superseded unseen. `verification` on a `message` is
/// about what this node knows, not what it found wrong: `unverified` means it
/// has not recalled the sender's identity, so `source` is a claim it cannot
/// check yet, not a proven forgery. `rejected` and `cancelled` are final
/// states like `failed`, but the router gives no reason for either — they
/// never carry `reason`/`retry`, and none should be inferred.
///
/// `requeue_refused` says a `resilum_lxmf_requeue_with_method` did not happen
/// and why. It is not a `delivery`: the message is wherever it was before the
/// call, so this never overrides a state the caller already has.
/// `reason:"not_found"` — the common one — means the router no longer holds the
/// id, which for a message the caller thought was in flight means it already
/// reached a terminal state and was already reported. Any other token is a
/// refusal to re-queue under the new method, and the message stays queued under
/// its old one. A re-queue the router did apply produces no event of its own,
/// in particular no `"state":"cancelled"` for the copy it took back out.
///
/// `announce` says a peer's LXMF delivery destination was heard from. Only
/// delivery destinations produce it — announces of this node's other
/// destinations never appear here — so `source` is always an address
/// `resilum_lxmf_send` accepts. `display_name` is what the peer put in its
/// announce and is present but empty when it named itself nothing; it is
/// self-asserted, unverified, and may be any UTF-8 the peer chose, so treat it
/// as a label to render, never as an identifier. `timestamp` is unix seconds
/// when this node heard the announce, not when the peer emitted it.
///
/// It is passive observation, not a liveness probe: a peer announces on its own
/// schedule — order-of-minutes at best — so absence over a shorter window says
/// nothing, and a fresh announce proves only that the announce reached some
/// interface, not that a message will. The queue is lossy by design: a missed
/// announce is superseded by the next rather than repeated, so do not count them.
///
/// Only `"state":"failed"` carries `reason` and `retry`, and always carries
/// both together. `retry` is this node's own verdict on what the caller should
/// do next and is the field to branch on — do not re-derive it from `reason`:
/// - `"resubmit"` — submit the same request again later, ideally with backoff.
///   Nothing about it was wrong.
/// - `"requeue"` — the router still holds this message and what blocked it was
///   the propagation path; call `resilum_lxmf_requeue_with_method` to move it
///   to a delivery method that can carry it.
/// - `"never"` — the request itself is malformed; repeating it verbatim cannot
///   succeed.
///
/// `reason` is a stable snake_case token naming what went wrong, for logs and
/// for messages shown to a person:
/// - `"attempts_exhausted"` — the peer did not accept delivery within the
///   router's retry budget (about a minute of direct attempts).
/// - `"queue_full"` — the outbound queue was full at submit time, which
///   typically drains within seconds.
/// - `"unsupported_method"` — the delivery method is not supported here (e.g.
///   paper delivery attempted through this call).
/// - `"identity_mismatch"` — the message id belongs to a different local
///   identity than the one operating on it now. A caller mistake, not a
///   transient condition.
/// - `"not_found"` — the router no longer holds this message id (already
///   delivered, cancelled, or never queued).
/// - `"no_wall_clock"` — this node has no usable wall-clock time yet, so it
///   cannot stamp the message with a timestamp a peer would accept.
/// - `"malformed_message"` — the message itself is bad: wrong format, wrong
///   destination, non-finite timestamp, and similar.
/// - `"propagation_node_unavailable"` — no propagation node is selected for a
///   propagated message.
/// - `"propagation_stamp_unavailable"` — the propagation path needed a proof
///   stamp that has not arrived yet.
/// - `"stale_stamp_request"` — a stamp result arrived for a message the queue
///   entry no longer matches.
/// - `"stale_build"` — a resource build no longer matches the queue entry it
///   was captured from.
/// - `"lxmf_node"`, `"propagation_failed"`, `"propagation_transport"`,
///   `"paper_format"`, `"stamp_failed"`, `"storage_failed"`,
///   `"corrupt_snapshot"` — an internal failure in the named area (node and
///   transport plumbing, propagation-node protocol, propagation transport,
///   paper-message handling, stamp generation, local storage, or persisted
///   router state). None maps to a single well-known cause, so a repeat is
///   worth reporting upstream even when `retry` says to try again.
///
/// Free with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_poll_event(node: *mut ResilumNode) -> *mut c_char {
    clear_error();
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        let Some(json) = node.0.lxmf().and_then(|lxmf| lxmf.next_event()) else {
            return std::ptr::null_mut();
        };
        match CString::new(json) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                set_error("event JSON contains an interior NUL");
                std::ptr::null_mut()
            }
        }
    })
}
