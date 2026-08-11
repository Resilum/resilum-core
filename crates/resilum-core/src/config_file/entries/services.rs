//! The i2p interface, egress listeners and the ingress hub.

use std::time::Duration;

use serde::Deserialize;

use crate::{EgressListen, I2pInterface, IngressConfig};

#[derive(Deserialize)]
pub(in crate::config_file) struct I2pFile {
    #[serde(default)]
    pub connectable: bool,
    #[serde(default)]
    pub peers: Vec<String>,
}

impl From<I2pFile> for I2pInterface {
    fn from(f: I2pFile) -> Self {
        Self {
            connectable: f.connectable,
            peers: f.peers,
        }
    }
}

#[derive(Deserialize)]
pub(in crate::config_file) struct EgressFile {
    service: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default = "wildcard")]
    exit_country: String,
    #[serde(default)]
    announce_interval_secs: Option<u64>,
}

impl From<EgressFile> for EgressListen {
    fn from(f: EgressFile) -> Self {
        let mut e = EgressListen::new(f.service, f.target);
        e.exit_country = f.exit_country;
        if let Some(secs) = f.announce_interval_secs {
            e.announce_interval = Duration::from_secs(secs);
        }
        e
    }
}

#[derive(Deserialize)]
pub(in crate::config_file) struct IngressFile {
    services: Vec<String>,
    listen_tcp: String,
    #[serde(default = "smart")]
    use_own: String,
    #[serde(default)]
    allow_countries: Vec<String>,
    #[serde(default)]
    deny_countries: Vec<String>,
}

impl From<IngressFile> for IngressConfig {
    fn from(f: IngressFile) -> Self {
        Self {
            services: f.services,
            listen_tcp: f.listen_tcp,
            use_own: f.use_own,
            allow_country: f.allow_countries,
            deny_country: f.deny_countries,
            target: None,
            probe_targets: Vec::new(),
        }
    }
}

fn wildcard() -> String {
    "*".into()
}

fn smart() -> String {
    "smart".into()
}
