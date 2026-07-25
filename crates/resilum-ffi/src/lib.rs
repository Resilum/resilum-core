//! resilum-ffi — C-ABI surface over `resilum-core` for the mobile app
//! (`dart:ffi`). Opaque-pointer wrappers; every entry point catches panics
//! (unwinding into C is undefined behaviour).

mod event;
mod node;
mod status;

pub use event::*;
pub use node::*;
pub use status::*;

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};

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

pub(crate) fn set_error(msg: impl Into<Vec<u8>>) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(msg).ok());
}

/// Run `body`, returning `default` if it panics (unwinding into C is UB).
pub(crate) fn guard<T>(default: T, body: impl FnOnce() -> T) -> T {
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
