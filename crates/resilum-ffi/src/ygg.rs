//! Yggdrasil transport attach/detach over the C ABI.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use resilum_core::YggHandle;

use crate::node::ResilumNode;
use crate::{guard, set_error};

/// An opaque Yggdrasil attachment handle.
pub struct ResilumYggdrasil(YggHandle);

/// Attach the Yggdrasil packet conduit `ygg_fd` (the gomobile engine's packet
/// channel), accepting RNS links arriving over ygg. `ygg_address` is the
/// engine's own address from `GetAddressString`, announced so peers know where
/// to dial. Returns an opaque handle, or null on error (see `resilum_last_error`).
/// The node must be running with yggdrasil discovery configured. The transport
/// takes ownership of `ygg_fd` and closes it on detach.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null; `ygg_fd` a
/// valid file descriptor the caller leaves to the transport; `ygg_address` a
/// valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_yggdrasil_attach(
    node: *const ResilumNode,
    ygg_fd: c_int,
    ygg_address: *const c_char,
) -> *mut ResilumYggdrasil {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        if ygg_address.is_null() {
            set_error("null ygg_address");
            return std::ptr::null_mut();
        }
        let Ok(address) = (unsafe { CStr::from_ptr(ygg_address) }).to_str() else {
            set_error("ygg_address is not valid UTF-8");
            return std::ptr::null_mut();
        };
        match node.0.ygg_attach(ygg_fd, address) {
            Ok(handle) => Box::into_raw(Box::new(ResilumYggdrasil(handle))),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// Detach the Yggdrasil transport, tearing down its links and closing the
/// conduit fd. Passing null is a no-op.
///
/// # Safety
/// `handle` must come from `resilum_yggdrasil_attach` and be detached at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_yggdrasil_detach(handle: *mut ResilumYggdrasil) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle) }.0.detach();
    }
}
