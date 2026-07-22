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

/// One enabled transport-discovery plugin. Peers announce where they accept
/// connections over this transport (`resilum.discovery.<service>`); consuming an
/// announce attaches a TCP client to reach them, optionally through a SOCKS5
/// proxy (Tor/I2P). Yggdrasil dials the announced IPv6 directly (`socks_proxy`
/// is `None`).
#[derive(Clone, Debug)]
pub struct DiscoveryService {
    pub service: String,
    /// Interface-name prefix for a discovered peer, e.g. `TorDiscovered`.
    pub name_prefix: String,
    /// Required host suffix an announced endpoint must carry (`.onion`,
    /// `.b32.i2p`); empty for a bracketed-IPv6 endpoint (Yggdrasil).
    pub host_suffix: String,
    /// SOCKS5 proxy every consumed peer is dialed through, or `None` to dial
    /// the announced host directly.
    pub socks_proxy: Option<(String, u16)>,
    /// File holding this node's own reachable address for this transport; read
    /// when producing our announce. `None` (or an unreadable path) means
    /// consume-only — we attach discovered peers but do not advertise ourselves.
    pub hostname_path: Option<PathBuf>,
    /// Port appended to the produced endpoint (our RNS listener).
    pub rns_port: u16,
}

impl DiscoveryService {
    /// Tor onion-service discovery: peers announced as `<onion>.onion:<port>`
    /// are dialed through the local Tor SOCKS5 proxy (127.0.0.1:9050).
    pub fn tor() -> Self {
        Self {
            service: "tor".into(),
            name_prefix: "TorDiscovered".into(),
            host_suffix: ".onion".into(),
            socks_proxy: Some(("127.0.0.1".into(), 9050)),
            hostname_path: None,
            rns_port: 4242,
        }
    }

    /// I2P b32-destination discovery: peers announced as
    /// `<b32>.b32.i2p:<port>` are dialed through the local i2pd SAM SOCKS5
    /// proxy (127.0.0.1:4447).
    pub fn i2p() -> Self {
        Self {
            service: "i2p".into(),
            name_prefix: "I2PDiscovered".into(),
            host_suffix: ".b32.i2p".into(),
            socks_proxy: Some(("127.0.0.1".into(), 4447)),
            hostname_path: None,
            rns_port: 4242,
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
    /// Transport-discovery plugins to run: each attaches discovered peers over
    /// its transport when their announce arrives.
    pub discovery: Vec<DiscoveryService>,
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
