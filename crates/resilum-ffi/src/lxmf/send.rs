//! Submitting an outbound message.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::guard;
use crate::node::ResilumNode;
use crate::set_error;

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
///
/// Returns as soon as the message is queued, not once it is sent: the returned
/// message id (hex) is what later `delivery` events are keyed by. Null on
/// error, with the reason in `resilum_last_error`. Free the returned string
/// with `resilum_string_free`.
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
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        let Some(json) = (unsafe { as_str(message_json) }) else {
            set_error("message_json is null or not utf-8");
            return std::ptr::null_mut();
        };
        let (Some(lxmf), Some(identity)) = (node.0.lxmf(), node.0.identity()) else {
            set_error("lxmf messaging is not enabled on this node");
            return std::ptr::null_mut();
        };
        // Built on this thread rather than inside the engine tick: packing and
        // signing are the expensive parts, and nothing here holds the core
        // lock. The router only has to accept a finished message.
        let source_hash = resilum_core::identity::lxmf_address(identity);
        let message = match resilum_core::lxmf::send::build_message(
            json,
            identity,
            source_hash,
            now_secs(),
        ) {
            Ok(message) => message,
            Err(e) => {
                set_error(e.as_str());
                return std::ptr::null_mut();
            }
        };
        let message_id = message.message_id;
        if let Err(e) = lxmf.submit(message) {
            set_error(e.to_string().as_str());
            return std::ptr::null_mut();
        }
        match CString::new(hex(&message_id)) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Seconds since the Unix epoch, as LXMF stamps a message's timestamp.
///
/// A device whose clock is not set will produce one its peers read as
/// implausible. The reference writes `time.time()` unvalidated and no peer
/// discards a message over it, so this does the same rather than withholding a
/// message over a clock we cannot fix.
fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or_default()
}

/// # Safety
/// `raw` must be a NUL-terminated string or null.
unsafe fn as_str<'a>(raw: *const c_char) -> Option<&'a str> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }.to_str().ok()
}
