//! Optional logging init: route `tracing` (arti's and ours) to the platform log.

use std::os::raw::c_int;
use std::sync::Once;

use resilum_core::letting_go::ItWasAlreadyThere as _;

use crate::{RESILUM_ERR_FAILED, RESILUM_OK, guard};

/// Install a `tracing` subscriber so arti's and our logs are visible — Android
/// logcat on Android, stderr elsewhere. Level from the `RUST_LOG` env var
/// (default `info`). Idempotent; returns `RESILUM_OK`.
#[unsafe(no_mangle)]
pub extern "C" fn resilum_logging_init() -> c_int {
    static ONCE: Once = Once::new();
    guard(RESILUM_ERR_FAILED, || {
        ONCE.call_once(init_subscriber);
        RESILUM_OK
    })
}

fn init_subscriber() {
    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    #[cfg(target_os = "android")]
    registry
        .with(paranoid_android::layer(env!("CARGO_PKG_NAME")))
        .try_init()
        .it_was_already_there();
    #[cfg(not(target_os = "android"))]
    registry
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .it_was_already_there();
    resilum_tasks::panics::are_told_to_the_log();
}
