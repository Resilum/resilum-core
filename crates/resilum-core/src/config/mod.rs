mod discovery;
mod egress;

pub use discovery::{DiscoveryService, EndpointFormat};
pub use egress::{ConnectConfig, EgressListen};

use std::path::PathBuf;
use std::time::Duration;

use crate::spec::Specs;

/// Default announce interval (600s), overridable by the
/// `RESILUM_BRIDGE_ANNOUNCE_INTERVAL` env var (seconds).
pub fn default_announce_interval() -> Duration {
    std::env::var("RESILUM_BRIDGE_ANNOUNCE_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(600))
}

/// Native I2P interface over the local i2pd SAM. Yggdrasil needs no dedicated
/// type: it is plain TCP over the overlay, reached via `listen`/`bootstrap`.
#[derive(Clone, Debug, Default)]
pub struct I2pInterface {
    pub connectable: bool,
    pub peers: Vec<String>,
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
    /// Transport-discovery plugins to run: each attaches discovered peers over
    /// its transport when their announce arrives.
    pub discovery: Vec<DiscoveryService>,
    /// How often the produce loop re-announces each discovery endpoint.
    /// A mobile client should shorten this or use the trigger API on
    /// network-change events.
    pub discovery_announce_interval: Duration,
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
            discovery: Vec::new(),
            discovery_announce_interval: default_announce_interval(),
            specs: Specs::default(),
        }
    }

    /// A bare node that joins the global mesh: default public + Yggdrasil
    /// anchors plus a discoverable listener.
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
