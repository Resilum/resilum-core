use std::ffi::CString;
use std::os::raw::c_char;

/// Free a string returned by this library (e.g. `resilum_node_status`).
///
/// # Safety
/// `s` must be a pointer returned by this library, or null. Do not free twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}
