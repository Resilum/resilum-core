//! resilum-ffi — C-ABI surface over `resilum-core` for an embedding caller.
//! Opaque-pointer wrappers; every entry point catches panics (unwinding into C
//! is undefined behaviour).

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub use self::ble::*;
pub use self::event::*;
pub use self::interface::*;
pub use self::iroh::*;
pub use self::logging::*;
pub use self::lxmf::*;
pub use self::node::*;
pub use self::platform::*;
pub use self::status::*;
pub use self::strings::*;
#[cfg(unix)]
pub use self::vpn::*;
#[cfg(unix)]
pub use self::wifi_group::*;
#[cfg(unix)]
pub use self::ygg::*;

mod ble;
mod event;
mod identity;
mod interface;
mod iroh;
mod logging;
mod lxmf;
mod node;
mod platform;
mod status;
mod strings;
#[cfg(unix)]
mod vpn;
#[cfg(unix)]
mod wifi_group;
#[cfg(unix)]
mod ygg;

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

pub(crate) fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn guard<T>(default: T, body: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            set_error(panic_message(payload.as_ref()));
            default
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic");
    format!("panic: {text}")
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
