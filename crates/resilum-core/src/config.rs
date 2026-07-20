use std::path::PathBuf;
use std::time::Duration;

use crate::spec::Specs;

/// Native I2P interface (leviculum `I2PInterface` over the local i2pd SAM at
/// 127.0.0.1:7656). Yggdrasil needs no dedicated type — bind `listen`/`bootstrap`
/// to Yggdrasil addresses (or `[::]`), it is plain TCP over the overlay.
#[derive(Clone, Debug, Default)]
pub struct I2pInterface {
    /// Open a local inbound endpoint, i.e. be reachable over I2P.
    pub connectable: bool,
    /// Remote `.b32.i2p` peers to dial.
    pub peers: Vec<String>,
}

/// Egress listen side: advertise `service` and forward inbound links to a local
/// TCP `target`.
#[derive(Clone, Debug)]
pub struct EgressListen {
    pub service: String,
    pub target: String,
    /// Advertised exit country; `*` means unknown.
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

/// Connect side: accept local TCP on `listen_tcp` and forward each connection
/// through the fastest eligible egress candidate for one of `services`.
#[derive(Clone, Debug)]
pub struct ConnectConfig {
    pub services: Vec<String>,
    pub listen_tcp: String,
    /// Own-candidate policy: `smart` (prefer others, fall back to own), `true`
    /// (allow own), `false` (never own).
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

/// Typed node configuration. Grows as the port progresses.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub instance_name: String,
    /// leviculum `storage_path`; state is not persisted when unset.
    pub storage_path: Option<PathBuf>,
    /// Public TCP listen address, e.g. `[::]:4242`.
    pub listen: Option<String>,
    /// `discovery_name` advertised on the public listener (needs `network_identity`).
    pub discovery_name: Option<String>,
    /// Private-discovery identity path; enables signed interface discovery.
    pub network_identity: Option<PathBuf>,
    /// Persistent anchor `host:port` addresses.
    pub bootstrap: Vec<String>,
    /// Bootstrap-only anchors (`bootstrap_only = yes`): used to learn paths.
    pub bootstrap_only: Vec<String>,
    /// Enable the local-segment AutoInterface.
    pub discover_interfaces: bool,
    /// Native I2P interface, when reachability over I2P is wanted.
    pub i2p: Option<I2pInterface>,
    /// Max discovered interfaces to auto-connect concurrently; `0` disables it.
    pub autoconnect_max: usize,
    /// Egress listen side; when set, run an exit endpoint.
    pub egress: Option<EgressListen>,
    /// Connect side; when set, accept local TCP and forward through egress.
    pub connect: Option<ConnectConfig>,
    /// Bridge/VPN/covert specs to run under supervision.
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
            egress: None,
            connect: None,
            specs: Specs::default(),
        }
    }

    /// The out-of-the-box network: a discoverable public listener plus the same
    /// public and Yggdrasil anchors the project ships, so a bare node
    /// joins the global mesh automatically.
    pub fn default_network(instance_name: impl Into<String>) -> Self {
        use crate::defaults;
        Self {
            listen: Some(defaults::DEFAULT_LISTEN.into()),
            discovery_name: Some(defaults::DISCOVERY_NAME.into()),
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
