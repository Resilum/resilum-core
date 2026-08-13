//! Platform handles the DNS resolver cannot obtain on its own.

use std::ffi::{c_int, c_void};

/// On Android, `vm` is the `JavaVM*` and `context` a **global** reference to
/// the application Context; a local one dies with the caller's frame. Without
/// them the resolver falls back to a public DNS server. Elsewhere it reads the
/// system configuration and this call does nothing. Calling twice is a no-op.
///
/// # Safety
/// On Android both pointers are handed to JNI unchecked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_platform_context_install(
    vm: *mut c_void,
    context: *mut c_void,
) -> c_int {
    crate::guard(crate::RESILUM_ERR_NULL, || {
        if vm.is_null() || context.is_null() {
            crate::set_error(String::from("platform context: null JavaVM or Context"));
            return crate::RESILUM_ERR_NULL;
        }
        #[cfg(target_os = "android")]
        {
            use std::sync::atomic::{AtomicBool, Ordering};

            static INSTALLED: AtomicBool = AtomicBool::new(false);

            // ndk_context asserts on a second install.
            if !INSTALLED.swap(true, Ordering::SeqCst) {
                unsafe { ndk_context::initialize_android_context(vm, context) };
            }
        }
        crate::RESILUM_OK
    })
}
