//! Node lifecycle over the C ABI.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use resilum_core::{Config, Node};

use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard, set_error};

/// Build a node from a YAML config; null on error (see `resilum_last_error`).
///
/// # Safety
/// `yaml` must be a valid NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_new_from_yaml(yaml: *const c_char) -> *mut Node {
    guard(std::ptr::null_mut(), || {
        if yaml.is_null() {
            set_error("null config");
            return std::ptr::null_mut();
        }
        let Ok(text) = (unsafe { CStr::from_ptr(yaml) }).to_str() else {
            set_error("config is not valid UTF-8");
            return std::ptr::null_mut();
        };
        let config: Config = match resilum_core::from_yaml(text) {
            Ok(config) => config,
            Err(e) => {
                set_error(e);
                return std::ptr::null_mut();
            }
        };
        match Node::new(config) {
            Ok(node) => Box::into_raw(Box::new(node)),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_start(node: *mut Node) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_ERR_NULL;
        };
        match node.start() {
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
pub unsafe extern "C" fn resilum_node_stop(node: *mut Node) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_ERR_NULL;
        };
        match node.stop() {
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
pub unsafe extern "C" fn resilum_node_is_running(node: *const Node) -> c_int {
    guard(0, || match unsafe { node.as_ref() } {
        Some(node) if node.is_running() => 1,
        _ => 0,
    })
}

/// # Safety
/// `node` must come from `resilum_node_new_from_yaml` and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_free(node: *mut Node) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}
