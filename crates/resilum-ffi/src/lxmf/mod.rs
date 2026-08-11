//! LXMF messaging surface — the contract the app builds against.

mod poll;
mod send;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

use crate::guard;
use crate::node::ResilumNode;
use crate::set_error;

/// 1 when this node can carry LXMF messages, else 0.
///
/// 0 means either that messaging is not configured, or that the node has not
/// registered its delivery destination yet — which takes one engine tick after
/// start. `resilum_lxmf_address` is valid either way.
///
/// # Safety
/// `node` must be a live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_lxmf_available(node: *const ResilumNode) -> c_int {
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
