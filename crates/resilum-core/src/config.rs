use std::path::PathBuf;
use std::time::Duration;

use crate::spec::Specs;

/// Native I2P interface over the local i2pd SAM. Yggdrasil needs no dedicated
/// type: it is plain TCP over the overlay, reached via `listen`/`bootstrap`.
#[derive(Clone, Debug, Default)]
pub struct I2pInterface {
    pub connectable: bool,
    pub peers: Vec<String>,
}

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
            announce_interval: Duration::from_secs(600),
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
}

impl ConnectConfig {
    pub fn new(service: impl Into<String>, listen_tcp: impl Into<String>) -> Self {
        Self {
            services: vec![service.into()],
            listen_tcp: listen_tcp.into(),
            use_own: "smart".into(),
            allow_country: Vec::new(),
            deny_country: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub instance_name: String,
    pub storage_path: Option<PathBuf>,
    pub listen: Option<String>,
    /// Needs `network_identity` to take effect.
    pub discovery_name: Option<String>,
    pub network_identity: Option<PathBuf>,
    pub bootstrap: Vec<String>,
    /// Rendered with `bootstrap_only = yes`.
    pub bootstrap_only: Vec<String>,
    pub discover_interfaces: bool,
    pub i2p: Option<I2pInterface>,
    /// `0` disables auto-connect.
    pub autoconnect_max: usize,
    pub egress: Vec<EgressListen>,
    pub connect: Option<ConnectConfig>,
    pub specs: Specs,
}

impl Config {
    pub fn minimal(instance_name: impl Into<String>) -> Self {
        Self {
            instance_name: instance_name.into(),
            storage_path: None,
            listen: None,
            discovery_name: None,
            network_identity: None,
            bootstrap: Vec::new(),
            bootstrap_only: Vec::new(),
            discover_interfaces: true,
            i2p: None,
            autoconnect_max: 5,
            egress: Vec::new(),
            connect: None,
            specs: Specs::default(),
        }
    }

    /// A bare node that joins the global mesh: the public and Yggdrasil anchors
    /// the project ships, plus a discoverable listener.
    pub fn default_network(instance_name: impl Into<String>) -> Self {
        use crate::defaults;
        Self {
            listen: Some(defaults::DEFAULT_LISTEN.into()),
            discovery_name: Some(defaults::DISCOVERY_NAME.into()),
            network_identity: Some(defaults::NETWORK_IDENTITY_FILE.into()),
            bootstrap: defaults::YGG_ANCHORS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            bootstrap_only: defaults::PUBLIC_ANCHORS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ..Self::minimal(instance_name)
        }
    }
}
