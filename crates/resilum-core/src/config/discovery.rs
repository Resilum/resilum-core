use std::path::PathBuf;

/// Wire format the announced endpoint uses. Consuming an announce parses it
/// and hands the host + port to `spawn_tcp_client`.
#[derive(Clone, Debug)]
pub enum EndpointFormat {
    /// `<host><suffix>:<port>`, host restricted to lowercase ASCII alnum + `.-`
    /// (Tor `.onion`, I2P `.b32.i2p`).
    Suffix(String),
    /// `[<ipv6>]:<port>` (Yggdrasil): the announced IPv6 is dialed directly.
    BracketedIpv6,
}

/// How to reach the transport's egress proxy.
#[derive(Clone, Debug)]
pub enum SocksProxy {
    /// An externally-running SOCKS5 proxy on this host.
    External(String, u16),
    /// In-process Tor via Arti — resolved to a local port at node start.
    EmbeddedArti,
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
    pub endpoint_format: EndpointFormat,
    /// SOCKS5 proxy every consumed peer is dialed through, or `None` to dial
    /// the announced host directly.
    pub socks_proxy: Option<SocksProxy>,
    /// File holding this node's own reachable address for this transport; read
    /// when producing our announce. `None` (or an unreadable path) means
    /// consume-only — we attach discovered peers but do not advertise ourselves.
    pub hostname_path: Option<PathBuf>,
    /// Port appended to the produced endpoint (our RNS listener).
    pub rns_port: u16,
}

impl DiscoveryService {
    /// Tor discovery via external `tor` daemon on `127.0.0.1:9050`.
    pub fn tor() -> Self {
        Self {
            service: "tor".into(),
            name_prefix: "TorDiscovered".into(),
            endpoint_format: EndpointFormat::Suffix(".onion".into()),
            socks_proxy: Some(SocksProxy::External("127.0.0.1".into(), 9050)),
            hostname_path: None,
            rns_port: 4242,
        }
    }

    /// Tor discovery via in-process Arti (requires `arti` feature).
    pub fn tor_embedded() -> Self {
        Self {
            socks_proxy: Some(SocksProxy::EmbeddedArti),
            ..Self::tor()
        }
    }

    /// I2P b32-destination discovery: peers announced as
    /// `<b32>.b32.i2p:<port>` are dialed through the local i2pd SAM SOCKS5
    /// proxy (127.0.0.1:4447).
    pub fn i2p() -> Self {
        Self {
            service: "i2p".into(),
            name_prefix: "I2PDiscovered".into(),
            endpoint_format: EndpointFormat::Suffix(".b32.i2p".into()),
            socks_proxy: Some(SocksProxy::External("127.0.0.1".into(), 4447)),
            hostname_path: None,
            rns_port: 4242,
        }
    }

    /// Yggdrasil discovery: peers announced as `[<ipv6>]:<port>` are dialed
    /// directly over the Yggdrasil overlay (no SOCKS proxy).
    pub fn yggdrasil() -> Self {
        Self {
            service: "yggdrasil".into(),
            name_prefix: "YggdrasilDiscovered".into(),
            endpoint_format: EndpointFormat::BracketedIpv6,
            socks_proxy: None,
            hostname_path: None,
            rns_port: 4242,
        }
    }
}
