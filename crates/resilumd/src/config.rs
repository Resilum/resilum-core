//! YAML config file for the daemon, mapped onto `resilum_core::Config`.

use std::path::Path;
use std::time::Duration;

use resilum_core::{Config, ConnectConfig, EgressListen, I2pInterface};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FileConfig {
    pub instance_name: String,
    #[serde(default)]
    pub storage_path: Option<std::path::PathBuf>,
    #[serde(default)]
    pub listen: Option<String>,
    /// Join the mesh through the built-in public/Yggdrasil anchors (parity).
    #[serde(default = "yes")]
    pub default_anchors: bool,
    /// Extra persistent anchors, added to the defaults.
    #[serde(default)]
    pub bootstrap: Vec<String>,
    #[serde(default = "yes")]
    pub discover: bool,
    #[serde(default)]
    pub i2p: Option<I2pFile>,
    #[serde(default)]
    pub egress: Option<EgressFile>,
    #[serde(default)]
    pub connect: Option<ConnectFile>,
}

#[derive(Deserialize)]
pub struct I2pFile {
    #[serde(default)]
    pub connectable: bool,
    #[serde(default)]
    pub peers: Vec<String>,
}

#[derive(Deserialize)]
pub struct EgressFile {
    pub service: String,
    pub target: String,
    #[serde(default = "wildcard")]
    pub exit_country: String,
    #[serde(default)]
    pub announce_interval_secs: Option<u64>,
}

#[derive(Deserialize)]
pub struct ConnectFile {
    pub services: Vec<String>,
    pub listen_tcp: String,
    #[serde(default = "smart")]
    pub use_own: String,
    #[serde(default)]
    pub allow_countries: Vec<String>,
    #[serde(default)]
    pub deny_countries: Vec<String>,
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

pub fn load(path: &Path) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let file: FileConfig =
        serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse config: {e}"))?;
    Ok(file.into_core())
}

impl FileConfig {
    pub fn into_core(self) -> Config {
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
        cfg.i2p = self.i2p.map(|i| I2pInterface {
            connectable: i.connectable,
            peers: i.peers,
        });
        cfg.egress = self.egress.map(|e| {
            let mut egress = EgressListen::new(e.service, e.target);
            egress.exit_country = e.exit_country;
            if let Some(secs) = e.announce_interval_secs {
                egress.announce_interval = Duration::from_secs(secs);
            }
            egress
        });
        cfg.connect = self.connect.map(|c| ConnectConfig {
            services: c.services,
            listen_tcp: c.listen_tcp,
            use_own: c.use_own,
            allow_country: c.allow_countries,
            deny_country: c.deny_countries,
        });
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_egress_and_connect() {
        let yaml = "
instance_name: node-a
default_anchors: false
listen: '[::]:4242'
bootstrap: [anchor.example:4343]
egress:
  service: socks-egress
  target: 127.0.0.1:1080
  exit_country: DE
connect:
  services: [socks-egress, tor]
  listen_tcp: 127.0.0.1:10808
";
        let cfg = serde_yaml_ng::from_str::<FileConfig>(yaml)
            .unwrap()
            .into_core();
        assert_eq!(cfg.instance_name, "node-a");
        assert_eq!(cfg.bootstrap, vec!["anchor.example:4343"]);
        let egress = cfg.egress.unwrap();
        assert_eq!(egress.service, "socks-egress");
        assert_eq!(egress.exit_country, "DE");
        let connect = cfg.connect.unwrap();
        assert_eq!(connect.services, vec!["socks-egress", "tor"]);
        assert_eq!(connect.use_own, "smart"); // default
    }

    #[test]
    fn bare_config_joins_the_default_network() {
        let cfg = serde_yaml_ng::from_str::<FileConfig>("instance_name: bare")
            .unwrap()
            .into_core();
        assert!(cfg.discover_interfaces);
        assert!(cfg.egress.is_none());
        assert!(!cfg.bootstrap.is_empty());
        assert!(!cfg.bootstrap_only.is_empty());
        assert!(cfg.listen.is_some());
    }
}
