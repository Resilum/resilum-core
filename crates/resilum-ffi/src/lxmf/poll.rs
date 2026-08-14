//! Draining inbound messages and delivery updates.

use std::ffi::CString;
use std::os::raw::c_char;

use crate::guard;
use crate::node::ResilumNode;

/// Next LXMF event as JSON, or null when the queue is empty. One of:
/// ```json
/// { "type":"message", "source":"<hex>", "message_id":"<hex>",
///   "timestamp": 0.0, "title_b64":"...", "content_b64":"...",
///   "fields": { "custom_type":"...", "custom_data": <value> } }
/// { "type":"delivery", "message_id":"<hex>", "state":"generating" | "queued" |
///   "sending" | "sent" | "awaiting_collection" | "delivered" | "rejected" |
///   "cancelled" | "failed" }
/// { "type":"overflow", "dropped": 0 }
/// ```
/// `delivery` tracks a message this node sent, keyed by the id
/// `resilum_lxmf_send` returned. `overflow` says how many events were discarded
/// because the queue filled up, which happens only if the app stops polling
/// while the mesh keeps delivering. Free with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_poll_event(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return std::ptr::null_mut();
        };
        let Some(json) = node.0.lxmf().and_then(|lxmf| lxmf.next_event()) else {
            return std::ptr::null_mut();
        };
        match CString::new(json) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
