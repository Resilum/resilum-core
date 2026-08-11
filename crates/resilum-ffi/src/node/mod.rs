//! Node lifecycle over the C ABI.

mod lifecycle;
mod runtime;

pub use lifecycle::*;
pub use runtime::*;

use std::ffi::CStr;
use std::os::raw::c_char;

use resilum_core::{Config, Node};

use crate::{guard, set_error};

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
