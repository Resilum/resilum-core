//! Shared scaffolding for the integration tests.
//!
//! Deliberately only what every one of them needs: each test binary compiles
//! this module separately, so anything one test does not call is dead code
//! there.

/// A storage directory unique to this test run, wiped if a previous run left
/// one behind — a stale identity there would change every destination hash.
pub fn temp_dir(prefix: &str, tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-{prefix}-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
