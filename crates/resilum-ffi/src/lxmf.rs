mod poll;
mod requeue;
mod send;

pub use poll::*;
pub use requeue::*;
pub use send::*;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use crate::guard;
use crate::node::ResilumNode;
use crate::set_error;

/// # Safety
/// `raw` must be a NUL-terminated string or null. The returned `&str` borrows
/// `raw`'s buffer, and its lifetime is whatever the call site infers — nothing
/// ties it to the C allocation. Use it only while `raw` is alive and unmodified,
/// and never return it past the entry point that received the pointer.
unsafe fn as_str<'a>(raw: *const c_char) -> Option<&'a str> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }.to_str().ok()
}

/// `None` before start.
pub(crate) fn address_hex(node: &ResilumNode) -> Option<String> {
    node.0.lxmf_address_hex()
}

/// 1 when this node can carry LXMF messages, else 0.
///
/// 0 means either that messaging is not configured, or that the node has not
/// registered its delivery destination yet — which takes one engine tick after
/// start. `resilum_lxmf_address` is valid either way.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_is_ready(node: *const ResilumNode) -> c_int {
    guard(0, || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return 0;
        };
        match node.0.lxmf() {
            Some(lxmf) if lxmf.is_ready() => 1,
            _ => 0,
        }
    })
}

/// How many messages the router still owes a verdict on, or 0 when messaging
/// is not configured.
///
/// Counts the queue restored from disk at start too, so a caller that was
/// killed mid-send can show what it is still carrying instead of only what this
/// run submitted. Sampled once per engine tick, so a just-submitted message may
/// not be in it yet.
///
/// `resilum_node_status`'s `lxmf.queued_ids` names the same messages this
/// counts, from the same sample.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_queued_count(node: *const ResilumNode) -> u64 {
    guard(0, || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return 0;
        };
        match node.0.lxmf() {
            Some(lxmf) => lxmf.outbound_depth() as u64,
            None => 0,
        }
    })
}

/// This node's LXMF destination hash, 32 hex chars, for others to message, or
/// null with the reason in `resilum_last_error`. Derived from the identity, so
/// it is available whether or not the messaging backend is active. Free with
/// `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_address(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        let Some(addr) = address_hex(node) else {
            set_error("node is not running or has no identity");
            return std::ptr::null_mut();
        };
        match CString::new(addr) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                set_error("lxmf address contains an interior NUL");
                std::ptr::null_mut()
            }
        }
    })
}
