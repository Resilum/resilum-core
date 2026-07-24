use std::time::Duration;

use super::default_announce_interval;

#[derive(Clone, Debug)]
pub struct EgressListen {
    pub service: String,
    /// External TCP forward target, or `None` to use the embedded SOCKS5
    /// backend (only valid for `service: socks-egress`).
    pub target: Option<String>,
    /// `*` means unknown.
    pub exit_country: String,
    pub announce_interval: Duration,
}

impl EgressListen {
    pub fn new(service: impl Into<String>, target: Option<String>) -> Self {
        Self {
            service: service.into(),
            target,
            exit_country: "*".into(),
            announce_interval: default_announce_interval(),
        }
    }
}
