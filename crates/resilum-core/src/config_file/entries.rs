//! Per-entry YAML types + their conversions to core config values.

use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::config::{IrohConfig, SocksProxy};
use crate::{DiscoveryService, EgressListen, I2pInterface, IngressConfig};

#[derive(Deserialize)]
pub(super) struct I2pFile {
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
pub(super) struct EgressFile {
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
pub(super) struct IngressFile {
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

#[derive(Deserialize)]
pub(super) struct DiscoveryFile {
    service: String,
    #[serde(default)]
    socks: Option<String>,
    #[serde(default)]
    hostname_path: Option<PathBuf>,
    #[serde(default)]
    rns_port: Option<u16>,
    // iroh is in-process, so — unlike the tor/i2p/ygg daemons with their own
    // config files — its optional knobs ride the discovery entry.
    #[serde(default)]
    relay: Option<String>,
    #[serde(default)]
    publish: bool,
    #[serde(default)]
    bootstrap: Vec<String>,
}

impl DiscoveryFile {
    pub(super) fn is_iroh(&self) -> bool {
        self.service == "iroh"
    }

    pub(super) fn into_iroh(self) -> IrohConfig {
        IrohConfig {
            relay: self.relay,
            publish: self.publish,
            bootstrap: self.bootstrap,
        }
    }
}

impl From<DiscoveryFile> for DiscoveryService {
    fn from(f: DiscoveryFile) -> Self {
        let mut svc = match f.service.as_str() {
            "tor" => DiscoveryService::tor(),
            "tor_embedded" => DiscoveryService::tor_embedded(),
            "i2p" => DiscoveryService::i2p(),
            "yggdrasil" => DiscoveryService::yggdrasil(),
            other => {
                tracing::warn!(service = %other, "unknown discovery service, falling back to tor()");
                DiscoveryService::tor()
            }
        };
        if let Some(spec) = f.socks {
            svc.socks_proxy = parse_socks(&spec);
        }
        if let Some(path) = f.hostname_path {
            svc.hostname_path = Some(path);
        }
        if let Some(port) = f.rns_port {
            svc.rns_port = port;
        }
        svc
    }
}

fn parse_socks(spec: &str) -> Option<SocksProxy> {
    let (host, port_str) = spec.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    Some(SocksProxy::External(
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned(),
        port,
    ))
}

fn wildcard() -> String {
    "*".into()
}
fn smart() -> String {
    "smart".into()
}
