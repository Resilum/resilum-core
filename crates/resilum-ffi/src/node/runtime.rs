//! What the app can ask of a node while it runs.

use std::os::raw::c_int;

use super::ResilumNode;
use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard};

/// Re-announce this node now, without waiting for the next interval: its
/// discovery endpoints and, when messaging is on, its LXMF delivery address.
///
/// Call it when the device's network changed. A peer cannot address a
/// destination whose announce it has never seen, so a fresh interface is
/// exactly when an announce is worth spending.
///
/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_announce_now(node: *const ResilumNode) {
    guard((), || {
        if let Some(node) = unsafe { node.as_ref() } {
            node.0.trigger_discovery_announce();
        }
    })
}

/// The bound local SOCKS port, or 0 if the connect listener is not up.
///
/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_socks_port(node: *const ResilumNode) -> u16 {
    guard(0, || {
        (unsafe { node.as_ref() }).map_or(0, |node| node.0.socks_port())
    })
}

/// Register a socket-protection callback, invoked with each outbound socket fd
/// before it connects (e.g. to bind it out of a captured tun). Pass null to
/// clear. Call before `resilum_node_start`. Returns `RESILUM_OK` or a negative
/// code.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null; `protect`, if
/// non-null, must stay valid for the node's lifetime.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_set_protect(
    node: *mut ResilumNode,
    protect: Option<extern "C" fn(std::os::raw::c_int)>,
) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_ERR_NULL;
        };
        let hook = protect.map(|cb| {
            std::sync::Arc::new(move |fd: std::os::fd::RawFd| cb(fd))
                as resilum_core::OutboundSocketHook
        });
        node.0.set_protect(hook);
        RESILUM_OK
    })
}
