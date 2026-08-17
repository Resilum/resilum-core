//! Installs rustls' process-default crypto provider once per process.
//!
//! `tokio-tungstenite`'s `wss://` path builds a `rustls::ClientConfig`
//! without ever installing a provider itself; rustls only resolves one
//! automatically when exactly one is compiled into the binary and nothing
//! installed one first — otherwise `ClientConfig::builder()` panics inside
//! whatever task happens to dial first. This crate always compiles one in,
//! so installing it explicitly here removes the dependence on nothing else
//! in the binary having done so already. A second install (some other crate
//! got there first) returns `Err` rather than panicking.

use std::sync::Once;

static INSTALL: Once = Once::new();

pub fn install_default_provider() {
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
