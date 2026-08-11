use std::os::raw::c_int;

use super::ResilumNode;
use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard, set_error};

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

/// # Safety
/// `node` must come from `resilum_node_new_from_yaml` and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_free(node: *mut ResilumNode) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}
