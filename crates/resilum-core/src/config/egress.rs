use std::time::Duration;

use super::default_announce_interval;

#[derive(Clone, Debug)]
pub struct EgressListen {
    pub service: String,
    pub target: String,
    /// `*` means unknown.
    pub exit_country: String,
    pub announce_interval: Duration,
}

impl EgressListen {
    pub fn new(service: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            target: target.into(),
            exit_country: "*".into(),
            announce_interval: default_announce_interval(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectConfig {
    pub services: Vec<String>,
    pub listen_tcp: String,
    /// `smart` | `true` | `false`.
    pub use_own: String,
    pub allow_country: Vec<String>,
    pub deny_country: Vec<String>,
    /// Explicit destination hash to dial, bypassing discovery.
    pub target: Option<[u8; 16]>,
    /// Custom probe targets (IPv4 literal `host:port`), override the env var
    /// and built-in defaults when non-empty.
    pub probe_targets: Vec<(String, u16)>,
}

impl ConnectConfig {
    pub fn new(service: impl Into<String>, listen_tcp: impl Into<String>) -> Self {
        Self {
            services: vec![service.into()],
            listen_tcp: listen_tcp.into(),
            use_own: "smart".into(),
            allow_country: Vec::new(),
            deny_country: Vec::new(),
            target: None,
            probe_targets: Vec::new(),
        }
    }
}
