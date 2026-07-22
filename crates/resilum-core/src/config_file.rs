//! YAML config, deserialized and mapped onto [`Config`]. Shared by the daemon
//! and the FFI so both accept the same format.

use std::time::Duration;

use serde::Deserialize;

use crate::{Config, ConnectConfig, EgressListen, I2pInterface};

/// Parse a YAML document into a [`Config`].
pub fn from_yaml(yaml: &str) -> Result<Config, String> {
    serde_yaml_ng::from_str::<FileConfig>(yaml)
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
    #[serde(default = "yes")]
    default_anchors: bool,
    #[serde(default)]
    bootstrap: Vec<String>,
    #[serde(default = "yes")]
    discover: bool,
    #[serde(default)]
    network_identity: Option<String>,
    #[serde(default)]
    i2p: Option<I2pFile>,
    #[serde(default)]
    egress: Vec<EgressFile>,
    #[serde(default)]
    connect: Option<ConnectFile>,
}

#[derive(Deserialize)]
struct I2pFile {
    #[serde(default)]
    connectable: bool,
    #[serde(default)]
    peers: Vec<String>,
}

#[derive(Deserialize)]
struct EgressFile {
    service: String,
    target: String,
    #[serde(default = "wildcard")]
    exit_country: String,
    #[serde(default)]
    announce_interval_secs: Option<u64>,
}

#[derive(Deserialize)]
struct ConnectFile {
    services: Vec<String>,
    listen_tcp: String,
    #[serde(default = "smart")]
    use_own: String,
    #[serde(default)]
    allow_countries: Vec<String>,
    #[serde(default)]
    deny_countries: Vec<String>,
}

fn yes() -> bool {
    true
}
fn wildcard() -> String {
    "*".into()
}
fn smart() -> String {
    "smart".into()
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
        cfg.bootstrap.extend(self.bootstrap);
        cfg.discover_interfaces = self.discover;
        if let Some(path) = self.network_identity {
            cfg.network_identity = Some(path.into());
        }
        cfg.i2p = self.i2p.map(|i| I2pInterface {
            connectable: i.connectable,
            peers: i.peers,
        });
        cfg.egress = self
            .egress
            .into_iter()
            .map(|e| {
                let mut egress = EgressListen::new(e.service, e.target);
                egress.exit_country = e.exit_country;
                if let Some(secs) = e.announce_interval_secs {
                    egress.announce_interval = Duration::from_secs(secs);
                }
                egress
            })
            .collect();
        cfg.connect = self.connect.map(|c| ConnectConfig {
            services: c.services,
            listen_tcp: c.listen_tcp,
            use_own: c.use_own,
            allow_country: c.allow_countries,
            deny_country: c.deny_countries,
            target: None,
            probe_targets: Vec::new(),
        });
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::from_yaml;

    #[test]
    fn maps_egress_and_connect() {
        let cfg = from_yaml(
            "
instance_name: node-a
default_anchors: false
listen: '[::]:4242'
bootstrap: [anchor.example:4343]
egress:
  - service: socks-egress
    target: 127.0.0.1:1080
    exit_country: DE
  - service: tor
    target: 127.0.0.1:9050
connect:
  services: [socks-egress, tor]
  listen_tcp: 127.0.0.1:10808
",
        )
        .unwrap();
        assert_eq!(cfg.instance_name, "node-a");
        assert_eq!(cfg.bootstrap, vec!["anchor.example:4343"]);
        assert_eq!(cfg.egress.len(), 2);
        assert_eq!(cfg.egress[0].exit_country, "DE");
        assert_eq!(cfg.egress[1].service, "tor");
        assert_eq!(cfg.connect.unwrap().use_own, "smart");
    }

    #[test]
    fn bare_config_joins_the_default_network() {
        let cfg = from_yaml("instance_name: bare").unwrap();
        assert!(cfg.egress.is_empty());
        assert!(!cfg.bootstrap.is_empty());
        assert!(cfg.listen.is_some());
    }
}
