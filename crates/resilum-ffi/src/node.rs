//! Node lifecycle over the C ABI.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use resilum_core::{Config, Node};

use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard, set_error};

/// An opaque node handle.
pub struct ResilumNode(pub(crate) Node);

/// Build a node from a YAML config; null on error (see `resilum_last_error`).
///
/// # Safety
/// `yaml` must be a valid NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_new_from_yaml(yaml: *const c_char) -> *mut ResilumNode {
    guard(std::ptr::null_mut(), || {
        node_from(yaml, resilum_core::from_yaml)
    })
}

/// Build a node from a JSON config; null on error (see `resilum_last_error`).
/// The mobile app builds this from a typed settings model — no YAML string
/// assembly on the Dart side.
///
/// # Safety
/// `json` must be a valid NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_new_from_json(json: *const c_char) -> *mut ResilumNode {
    guard(std::ptr::null_mut(), || {
        node_from(json, resilum_core::from_json)
    })
}

/// Shared body: decode the C string, parse it with `parse`, build the node.
fn node_from(
    text: *const c_char,
    parse: impl FnOnce(&str) -> Result<Config, String>,
) -> *mut ResilumNode {
    if text.is_null() {
        set_error("null config");
        return std::ptr::null_mut();
    }
    let Ok(text) = (unsafe { CStr::from_ptr(text) }).to_str() else {
        set_error("config is not valid UTF-8");
        return std::ptr::null_mut();
    };
    let config: Config = match parse(text) {
        Ok(config) => config,
        Err(e) => {
            set_error(e);
            return std::ptr::null_mut();
        }
    };
    match Node::new(config) {
        Ok(node) => Box::into_raw(Box::new(ResilumNode(node))),
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_start(node: *mut ResilumNode) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_ERR_NULL;
        };
        match node.0.start() {
            Ok(()) => RESILUM_OK,
            Err(e) => {
                set_error(e.to_string());
                RESILUM_ERR_FAILED
            }
        }
    })
}

/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_stop(node: *mut ResilumNode) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_ERR_NULL;
        };
        match node.0.stop() {
            Ok(()) => RESILUM_OK,
            Err(e) => {
                set_error(e.to_string());
                RESILUM_ERR_FAILED
            }
        }
    })
}

/// 1 if running, else 0.
///
/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_is_running(node: *const ResilumNode) -> c_int {
    guard(0, || match unsafe { node.as_ref() } {
        Some(node) if node.0.is_running() => 1,
        _ => 0,
    })
}

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

/// # Safety
/// `node` must come from `resilum_node_new_from_yaml` and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_free(node: *mut ResilumNode) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}
