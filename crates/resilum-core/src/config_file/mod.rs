//! YAML config, deserialized and mapped onto [`Config`]. Shared by the daemon
//! and the FFI so both accept the same format.

mod entries;
mod env_expand;

use serde::Deserialize;

use crate::Config;
use entries::{IngressFile, DiscoveryFile, EgressFile, I2pFile};

pub fn from_yaml(yaml: &str) -> Result<Config, String> {
    let expanded = env_expand::expand(yaml);
    serde_yaml_ng::from_str::<FileConfig>(&expanded)
        .map_err(|e| format!("parse config: {e}"))
        .map(FileConfig::into_core)
}

#[derive(Deserialize)]
struct FileConfig {
    instance_name: String,
    #[serde(default)]
    storage_path: Option<std::path::PathBuf>,
    #[serde(default)]
    listen: Option<String>,
    #[serde(default)]
    reachable_on: Option<String>,
    #[serde(default = "yes")]
    default_anchors: bool,
    #[serde(default)]
    bootstrap: Vec<String>,
    #[serde(default = "yes")]
    discover_interfaces: bool,
    #[serde(default)]
    network_identity: Option<String>,
    #[serde(default)]
    i2p: Option<I2pFile>,
    #[serde(default)]
    egress: Vec<EgressFile>,
    #[serde(default)]
    ingress: Option<IngressFile>,
    #[serde(default)]
    advertised_mirrors: Vec<String>,
    #[serde(default)]
    rngit_destination_file: Option<std::path::PathBuf>,
    #[serde(default)]
    discovery: Vec<DiscoveryFile>,
}

fn yes() -> bool {
    true
}

impl FileConfig {
    fn into_core(self) -> Config {
        let mut cfg = if self.default_anchors {
            Config::default_network(self.instance_name)
        } else {
            Config::minimal(self.instance_name)
        };
        cfg.storage_path = self.storage_path;
        if self.listen.is_some() {
            cfg.listen = self.listen;
        }
        if self.reachable_on.is_some() {
            cfg.reachable_on = self.reachable_on;
        }
        cfg.bootstrap.extend(self.bootstrap);
        cfg.discover_interfaces = self.discover_interfaces;
        if let Some(path) = self.network_identity {
            cfg.network_identity = Some(path.into());
        }
        cfg.i2p = self.i2p.map(Into::into);
        cfg.egress = self.egress.into_iter().map(Into::into).collect();
        cfg.ingress = self.ingress.map(Into::into);
        cfg.advertised_mirrors = self.advertised_mirrors;
        cfg.rngit_destination_file = self.rngit_destination_file;
        if !self.discovery.is_empty() {
            cfg.discovery = self.discovery.into_iter().map(Into::into).collect();
        }
        cfg
    }
}

#[cfg(test)]
mod tests;
