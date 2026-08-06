//! LXMF messaging surface — the contract the app builds against.
//!
//! These entry points are stable. `resilum_lxmf_address` is live (it derives
//! from the identity); send/poll are stubbed until the messaging backend is
//! wired in, reporting "not ready" via `resilum_last_error` and leaving
//! `resilum_lxmf_available` at 0. The app can build the full flow now and it
//! lights up when the backend lands — no signature change.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

use crate::guard;
use crate::node::ResilumNode;
use crate::set_error;

/// 1 when the LXMF messaging backend is active on this node, else 0.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_available(node: *const ResilumNode) -> c_int {
    guard(0, || {
        let _ = node;
        0
    })
}

/// This node's LXMF destination hash (hex) for others to message, or null.
/// Derived from the identity, so it is available whether or not the messaging
/// backend is active. Free with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_address(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return std::ptr::null_mut();
        };
        let Some(addr) = node.0.lxmf_address() else {
            set_error("node is not running or has no identity");
            return std::ptr::null_mut();
        };
        match CString::new(addr) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Submit an LXMF message. `message_json`:
/// ```json
/// {
///   "dest": "<32-hex LXMF destination>",
///   "method": "direct" | "opportunistic" | "propagated",
///   "title_b64": "<base64, optional>",
///   "content_b64": "<base64 human-readable body, optional>",
///   "fields": {
///     "custom_type": "<app schema id, e.g. rcb/1>",
///     "custom_data": <any structured value>
///   }
/// }
/// ```
/// `content_b64` is the human-readable body (chat text). Machine payloads (RPC
/// requests, app events) go in `fields`: `custom_type` maps to LXMF
/// FIELD_CUSTOM_TYPE (0xFB) and `custom_data` to FIELD_CUSTOM_DATA (0xFC),
/// msgpack-encoded on the wire so structured and byte values stay compact.
/// Everything except `dest` and `method` is optional.
/// Returns the message id (hex) for correlating delivery, or null on error.
/// Free the returned string with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null; `message_json` a NUL-terminated string
/// or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_send(
    node: *const ResilumNode,
    message_json: *const c_char,
) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let _ = (node, message_json);
        set_error("lxmf backend not yet integrated");
        std::ptr::null_mut()
    })
}

/// Next LXMF event as JSON, or null when the queue is empty. Either an inbound
/// message or a delivery-state update for a message we sent:
/// ```json
/// { "type":"message", "source":"<hex>", "message_id":"<hex>",
///   "timestamp": 0.0, "title_b64":"...", "content_b64":"...",
///   "fields": { "custom_type":"...", "custom_data": <value> } }
/// { "type":"delivery", "message_id":"<hex>",
///   "state":"sent" | "delivered" | "failed" | "propagated" }
/// ```
/// Free with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_poll(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let _ = node;
        std::ptr::null_mut()
    })
}
