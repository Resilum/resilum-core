use std::sync::Arc;

use crate::ble::radio::Radio;

#[cfg(feature = "ble")]
pub(super) async fn opened() -> Option<Arc<dyn Radio>> {
    use futures::FutureExt;

    let opening = std::panic::AssertUnwindSafe(crate::ble::backend::BlewRadio::open_or_say_why());
    match opening.catch_unwind().await {
        Ok(Ok(radio)) => Some(Arc::new(radio)),
        Ok(Err(e)) => {
            tracing::warn!(error = ?e, "no ble radio on this host");
            None
        }
        Err(panic) => {
            tracing::warn!(
                said = what_it_said(&panic),
                "opening the ble radio panicked, so this host has none"
            );
            None
        }
    }
}

#[cfg(feature = "ble")]
fn what_it_said(panic: &Box<dyn std::any::Any + Send>) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("nothing")
}

#[cfg(not(feature = "ble"))]
pub(super) async fn opened() -> Option<Arc<dyn Radio>> {
    None
}
