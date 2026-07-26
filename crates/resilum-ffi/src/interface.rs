//! Runtime interface attach/detach over the C ABI.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use crate::node::ResilumNode;
use crate::{RESILUM_ERR_FAILED, RESILUM_ERR_NULL, RESILUM_OK, guard, set_error};

/// Attach an interface from a leviculum interface-config JSON. Returns a JSON
/// string `{"ids":[...]}` with the assigned interface ids (several for a
/// fan-out type), or null on error (see `resilum_last_error`). Free the string
/// with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null; `config_json`
/// a valid NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_interface_add(
    node: *const ResilumNode,
    config_json: *const c_char,
) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        if config_json.is_null() {
            set_error("null interface config");
            return std::ptr::null_mut();
        }
        let Ok(json) = (unsafe { CStr::from_ptr(config_json) }).to_str() else {
            set_error("interface config is not valid UTF-8");
            return std::ptr::null_mut();
        };
        match node.0.add_interface(json) {
            Ok(ids) => match CString::new(serde_json::json!({ "ids": ids }).to_string()) {
                Ok(c) => c.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// Detach an interface by id. Returns `RESILUM_OK` (0), or a negative code on
/// error (see `resilum_last_error`). Removing an unknown id succeeds (no-op).
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_interface_remove(node: *const ResilumNode, id: u64) -> c_int {
    guard(RESILUM_ERR_FAILED, || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return RESILUM_ERR_NULL;
        };
        match node.0.remove_interface(id) {
            Ok(()) => RESILUM_OK,
            Err(e) => {
                set_error(e.to_string());
                RESILUM_ERR_FAILED
            }
        }
    })
}
