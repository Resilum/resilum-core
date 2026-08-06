//! iroh transport attach/detach over the C ABI.

use std::ffi::CString;
use std::os::raw::c_char;

use resilum_core::IrohHandle;

use crate::node::ResilumNode;
use crate::{guard, set_error};

/// An opaque iroh attachment handle.
pub struct ResilumIroh(IrohHandle);

/// Attach the in-process iroh transport: bind the endpoint, accept inbound RNS
/// links, and dial the configured `[iroh] bootstrap` peers. Returns an opaque
/// handle, or null on error (see `resilum_last_error`). The node must be running
/// with an `[iroh]` config.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_iroh_attach(node: *const ResilumNode) -> *mut ResilumIroh {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        match node.0.iroh_attach() {
            Ok(handle) => Box::into_raw(Box::new(ResilumIroh(handle))),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// This node's iroh `EndpointId` — the address peers dial us on — or null on
/// error. Free with `resilum_string_free`.
///
/// # Safety
/// `handle` must come from `resilum_iroh_attach` or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_iroh_node_id(handle: *const ResilumIroh) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return std::ptr::null_mut();
        };
        match CString::new(handle.0.endpoint_id()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Detach the iroh transport, tearing down its links. Passing null is a no-op.
///
/// # Safety
/// `handle` must come from `resilum_iroh_attach` and be detached at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_iroh_detach(handle: *mut ResilumIroh) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle) }.0.detach();
    }
}
