//! Routing-hub attach/detach over the C ABI.

use std::os::raw::c_int;

use resilum_core::VpnHandle;

use crate::node::ResilumNode;
use crate::{guard, set_error};

/// An opaque routing-hub attachment handle.
pub struct ResilumVpn(VpnHandle);

/// Attach an L3 routing hub to `tun_fd`, forwarding its TCP flows through the
/// egress mesh; `mtu` is the tun's L3 MTU. Returns an opaque handle, or null on
/// error (see `resilum_last_error`). The node must be running with an ingress
/// policy. The hub takes ownership of `tun_fd` and closes it on detach.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null; `tun_fd` a
/// valid file descriptor the caller then leaves to the hub.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_vpn_attach(
    node: *const ResilumNode,
    tun_fd: c_int,
    mtu: usize,
) -> *mut ResilumVpn {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        match node.0.vpn_attach(tun_fd, mtu) {
            Ok(handle) => Box::into_raw(Box::new(ResilumVpn(handle))),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// Detach a routing hub, tearing down its tasks and closing the tun fd. Passing
/// null is a no-op.
///
/// # Safety
/// `handle` must come from `resilum_vpn_attach` and be detached at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_vpn_detach(handle: *mut ResilumVpn) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle) }.0.detach();
    }
}
