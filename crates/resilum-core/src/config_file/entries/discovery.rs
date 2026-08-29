use std::path::PathBuf;

use serde::Deserialize;

use crate::DiscoveryService;
use crate::config::{IrohConfig, SocksProxy};

#[derive(Deserialize)]
pub(in crate::config_file) struct DiscoveryFile {
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
    pub(in crate::config_file) fn is_iroh(&self) -> bool {
        self.service == "iroh"
    }

    pub(in crate::config_file) fn is_yggdrasil(&self) -> bool {
        self.service == "yggdrasil"
    }

    pub(in crate::config_file) fn into_iroh(self) -> IrohConfig {
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
