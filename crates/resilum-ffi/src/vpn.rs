//! VPN attach/detach over the C ABI.

use std::os::raw::c_int;

use resilum_core::VpnHandle;

use crate::node::ResilumNode;
use crate::{guard, set_error};

pub struct ResilumVpn(VpnHandle);

/// Attach the device's L3 traffic on `tun_fd`, forwarding its TCP flows through
/// the egress mesh; `mtu` is the tun's L3 MTU. `ygg_fd` is the packet fd of a
/// host-managed Yggdrasil conduit (`200::/7` is routed to it); pass a negative
/// value for none. Returns an opaque handle, or null on error (see
/// `resilum_last_error`). The node must be running with an ingress policy.
/// Ownership of `tun_fd` (and `ygg_fd`) passes here, and both are closed on
/// detach.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null; `tun_fd` (and
/// `ygg_fd`, if non-negative) valid file descriptors the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_vpn_attach(
    node: *const ResilumNode,
    tun_fd: c_int,
    mtu: usize,
    ygg_fd: c_int,
) -> *mut ResilumVpn {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        match node
            .0
            .vpn_attach(tun_fd, mtu, (ygg_fd >= 0).then_some(ygg_fd))
        {
            Ok(handle) => Box::into_raw(Box::new(ResilumVpn(handle))),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// Tears down the attachment's tasks and closes the tun fd. Passing null is a
/// no-op.
///
/// # Safety
/// `handle` must come from `resilum_vpn_attach` and be detached at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_vpn_detach(handle: *mut ResilumVpn) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle) }.0.detach();
    }
}
