//! resilum-ffi — C-ABI surface over `resilum-core` for the mobile app
//! (`dart:ffi`). Opaque-pointer wrappers around the node lifecycle. Every entry
//! point catches panics (unwinding into C is undefined behaviour).

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};

use resilum_core::{Config, Event, Node};

pub const RESILUM_OK: c_int = 0;
pub const RESILUM_ERR_NULL: c_int = -1;
pub const RESILUM_ERR_FAILED: c_int = -2;

pub const RESILUM_EVENT_NONE: c_int = 0;
pub const RESILUM_EVENT_STARTED: c_int = 1;
pub const RESILUM_EVENT_STOPPED: c_int = 2;
pub const RESILUM_EVENT_PEER_DISCOVERED: c_int = 3;
pub const RESILUM_EVENT_RECEIVED: c_int = 4;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<Vec<u8>>) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(msg).ok());
}

fn guard<T>(default: T, body: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(default)
}

/// The last error on this thread as a NUL-terminated string, or null if none.
/// Valid until the next failing call on the same thread; copy it if you keep it.
#[unsafe(no_mangle)]
pub extern "C" fn resilum_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr())
    })
}

/// Build a node from a YAML config. Returns an opaque pointer to release with
/// `resilum_node_free`, or null on error (see `resilum_last_error`).
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

/// 1 if the node is running, else 0.
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

/// Pop the next queued lifecycle event as a `RESILUM_EVENT_*` code, or
/// `RESILUM_EVENT_NONE` when the queue is empty.
///
/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_poll_event(node: *mut Node) -> c_int {
    guard(RESILUM_EVENT_NONE, || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return RESILUM_EVENT_NONE;
        };
        match node.poll_event() {
            Some(Event::Started) => RESILUM_EVENT_STARTED,
            Some(Event::Stopped) => RESILUM_EVENT_STOPPED,
            Some(Event::PeerDiscovered(_)) => RESILUM_EVENT_PEER_DISCOVERED,
            Some(Event::Received { .. }) => RESILUM_EVENT_RECEIVED,
            None => RESILUM_EVENT_NONE,
        }
    })
}

/// Release a node. Passing null is a no-op.
///
/// # Safety
/// `node` must come from `resilum_node_new_from_yaml` and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_free(node: *mut Node) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}
