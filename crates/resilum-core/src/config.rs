mod ble;
mod covert_discovery;
mod discovery;
mod egress;
mod how_a_node_starts;
mod ingress;
mod lxmf;
mod udp;
mod wifi_group;

pub use ble::BleInterface;
pub use covert_discovery::CovertDiscoveryService;
pub use discovery::{DiscoveryService, EndpointFormat, SocksProxy};
pub use egress::EgressListen;
pub use ingress::IngressConfig;
pub use lxmf::LxmfConfig;
pub use udp::UdpInterface;
pub use wifi_group::WifiGroup;

use std::path::PathBuf;
use std::time::Duration;

use crate::discovery::Service;
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

/// In-process iroh QUIC transport (NAT hole-punching, relay fallback). Presence
/// enables it. `bootstrap` is `NodeId(+relay)` strings dialled at startup to
/// form an RNS interface when other transports can't reach an anchor.
#[derive(Clone, Debug, Default)]
pub struct IrohConfig {
    /// Relay for hole-punch coordination. `None` uses the built-in public
    /// relays; `Some` is a custom relay URL.
    pub relay: Option<String>,
    /// Publish our `NodeId → address` to a public directory. Off by default so
    /// a leaf never beacons its address; an anchor may turn it on.
    pub publish: bool,
    pub bootstrap: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub instance_name: String,
    pub storage_path: Option<PathBuf>,
    pub listen: Option<String>,
    /// Address peers should dial back on. Set when the address peers reach us
    /// on differs from what `listen` binds locally — 1:1 NAT, port forwarding,
    /// dual-stack hosts, VPN overlays. `None` → advertise the `listen` address.
    pub reachable_on: Option<String>,
    /// Needs `network_identity` to take effect.
    pub discovery_name: Option<String>,
    pub network_identity: Option<PathBuf>,
    /// Base64 raw-64-byte RNS private identity to run as, never persisted (the
    /// caller owns it). `None` loads or generates one under `storage_path`.
    pub identity_private_base64: Option<String>,
    pub bootstrap: Vec<String>,
    /// Rendered with `bootstrap_only = yes`.
    pub bootstrap_only: Vec<String>,
    pub discover_interfaces: bool,
    pub udp: Option<UdpInterface>,
    pub ble: Option<BleInterface>,
    pub wifi_group: Option<WifiGroup>,
    pub i2p: Option<I2pInterface>,
    pub iroh: Option<IrohConfig>,
    pub lxmf: Option<LxmfConfig>,
    /// `0` disables auto-connect.
    pub autoconnect_max: usize,
    pub egress: Vec<EgressListen>,
    pub ingress: Option<IngressConfig>,
    /// Canonical repo names this node hosts as rngit mirrors — advertised in
    /// the mesh so downloaders can discover them without knowing rns URLs.
    pub advertised_mirrors: Vec<String>,
    /// File the rngit sidecar writes its Repositories Destination hash into,
    /// read at announce time to embed the mirror URL peers should dial.
    pub rngit_destination_file: Option<PathBuf>,
    /// Transport-discovery plugins to run: each attaches discovered peers over
    /// its transport when their announce arrives.
    pub discovery: Vec<DiscoveryService>,
    /// Covert-carrier discovery plugins (parallel to `discovery`).
    pub covert_discovery: Vec<CovertDiscoveryService>,
    /// How often the produce loop re-announces each discovery endpoint. A
    /// caller whose connectivity changes often should shorten it, or announce
    /// on demand with `Node::trigger_discovery_announce`.
    pub discovery_announce_interval: Duration,
    pub advertised_services: Vec<Service>,
    pub specs: Specs,
}
