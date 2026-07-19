//! resilum-ffi — C-ABI surface over `resilum-core` for the mobile app
//! (`dart:ffi` / `flutter_rust_bridge`).
//!
//! Thin, opaque-pointer wrappers around `Node`, mirroring the node lifecycle.
//! Transport calls are added as the core grows. Skeleton.

use resilum_core::{Config, Node};
use std::os::raw::c_int;

/// Return codes for the C ABI.
pub const RESILUM_OK: c_int = 0;
pub const RESILUM_ERR_NULL: c_int = -1;
pub const RESILUM_ERR_FAILED: c_int = -2;

/// Create a node with a minimal default config. Returns an opaque pointer that
/// must be released with `resilum_node_free`. Returns null on failure.
#[unsafe(no_mangle)]
pub extern "C" fn resilum_node_new() -> *mut Node {
    match Node::new(Config::minimal("mobile")) {
        Ok(node) => Box::into_raw(Box::new(node)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Start the node.
///
/// # Safety
/// `node` must be a pointer returned by `resilum_node_new` and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_start(node: *mut Node) -> c_int {
    let Some(node) = (unsafe { node.as_mut() }) else {
        return RESILUM_ERR_NULL;
    };
    match node.start() {
        Ok(()) => RESILUM_OK,
        Err(_) => RESILUM_ERR_FAILED,
    }
}

/// Stop the node.
///
/// # Safety
/// `node` must be a valid pointer from `resilum_node_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_stop(node: *mut Node) -> c_int {
    let Some(node) = (unsafe { node.as_mut() }) else {
        return RESILUM_ERR_NULL;
    };
    match node.stop() {
        Ok(()) => RESILUM_OK,
        Err(_) => RESILUM_ERR_FAILED,
    }
}

/// Release a node created by `resilum_node_new`. Passing null is a no-op.
///
/// # Safety
/// `node` must be a pointer from `resilum_node_new`, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_free(node: *mut Node) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}
