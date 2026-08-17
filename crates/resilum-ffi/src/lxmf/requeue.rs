//! Re-queuing an outbound message under a different delivery method.

use std::os::raw::{c_char, c_int};

use super::as_str;
use crate::node::ResilumNode;
use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard, set_error};

/// Re-queue a message the router is still carrying under a different delivery
/// method. `message_id` is the hex id `resilum_lxmf_send` returned; `method` is
/// `"opportunistic" | "direct" | "propagated"`.
///
/// This is how a message moves between delivery paths — a direct send nobody
/// answered onto a propagation node, which holds it until the recipient next
/// comes online to collect it. Calling `resilum_lxmf_send` again cannot do it:
/// an LXMF message id is a hash over the destination, source and payload, and
/// carries nothing of the delivery method, so the second submit arrives as a
/// duplicate of the copy already queued and is dropped.
///
/// **The message id does not change.** Delivery events keep arriving under the
/// id the first send returned.
///
/// Returns `RESILUM_OK` once the request is queued, not once it is applied:
/// like `resilum_lxmf_send`, the router sees it a tick later.
///
/// **When the id names nothing the router holds** — already delivered, failed,
/// cancelled, or never queued — nothing is re-queued and a `requeue_refused`
/// event arrives through `resilum_lxmf_poll_event`, which documents its
/// `reason`/`retry`. A refused re-queue never loses the message: it stays
/// queued under the method it already had. A successful one emits no event.
///
/// Returns `RESILUM_ERR_NULL` for a null node, `RESILUM_ERR_FAILED` for a
/// malformed id or method, or when messaging is not enabled on this node, with
/// the reason in `resilum_last_error`.
///
/// # Safety
/// `node` must be a live handle or null; `message_id` and `method` NUL-
/// terminated strings or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_requeue_with_method(
    node: *const ResilumNode,
    message_id: *const c_char,
    method: *const c_char,
) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return RESILUM_ERR_NULL;
        };
        let (Some(message_id), Some(method)) =
            (unsafe { as_str(message_id) }, unsafe { as_str(method) })
        else {
            set_error("message_id or method is null or not utf-8");
            return RESILUM_ERR_FAILED;
        };
        let (message_id, method) =
            match resilum_core::lxmf::requeue::parse_request(message_id, method) {
                Ok(parsed) => parsed,
                Err(e) => {
                    set_error(e.to_string());
                    return RESILUM_ERR_FAILED;
                }
            };
        let Some(lxmf) = node.0.lxmf() else {
            set_error("lxmf messaging is not enabled on this node");
            return RESILUM_ERR_FAILED;
        };
        match lxmf.requeue_with_method(message_id, method) {
            Ok(()) => RESILUM_OK,
            Err(e) => {
                set_error(e.to_string().as_str());
                RESILUM_ERR_FAILED
            }
        }
    })
}
