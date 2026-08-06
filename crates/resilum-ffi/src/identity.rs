//! Identity helpers over the C ABI. Each returns a JSON `*mut c_char` freed with
//! `resilum_string_free`; errors surface via `resilum_last_error`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde::Serialize;

use crate::{guard, set_error};

#[derive(Serialize)]
struct Generated {
    private_base64: String,
    identity_hash: String,
    lxmf_address: String,
}

#[derive(Serialize)]
struct Hashes {
    identity_hash: String,
    lxmf_address: String,
}

/// A fresh identity as `{ private_base64, identity_hash, lxmf_address }`, or null
/// on error.
#[unsafe(no_mangle)]
pub extern "C" fn resilum_identity_generate() -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let id = resilum_core::identity::generate();
        let Some(private_base64) = resilum_core::identity::to_base64(&id) else {
            set_error("identity has no exportable private key");
            return std::ptr::null_mut();
        };
        json_or_null(&Generated {
            private_base64,
            identity_hash: resilum_core::identity::identity_hash_hex(&id),
            lxmf_address: resilum_core::identity::lxmf_address_hex(&id),
        })
    })
}

/// Hashes for a base64 private blob as `{ identity_hash, lxmf_address }`, or null
/// if the blob is not a valid 64-byte RNS identity.
///
/// # Safety
/// `private_base64` must be a valid NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_identity_hash(private_base64: *const c_char) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        if private_base64.is_null() {
            set_error("private_base64 is null");
            return std::ptr::null_mut();
        }
        let Ok(b64) = (unsafe { CStr::from_ptr(private_base64) }).to_str() else {
            set_error("private_base64 is not UTF-8");
            return std::ptr::null_mut();
        };
        let Some(id) = resilum_core::identity::from_base64(b64) else {
            set_error("private_base64 is not a valid 64-byte RNS identity");
            return std::ptr::null_mut();
        };
        json_or_null(&Hashes {
            identity_hash: resilum_core::identity::identity_hash_hex(&id),
            lxmf_address: resilum_core::identity::lxmf_address_hex(&id),
        })
    })
}

fn json_or_null<T: Serialize>(value: &T) -> *mut c_char {
    match serde_json::to_string(value)
        .ok()
        .and_then(|s| CString::new(s).ok())
    {
        Some(c) => c.into_raw(),
        None => std::ptr::null_mut(),
    }
}
