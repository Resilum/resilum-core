//! Local network helpers reused across transports (discovery, egress).

mod routable;
mod yggdrasil;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

pub use routable::{is_globally_routable, names_one_host};
pub use yggdrasil::yggdrasil_local_ipv6;

const V4_PROBE: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 80);
const V6_PROBE: SocketAddr = SocketAddr::new(
    IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
    80,
);

/// Detect the host's globally-routable IPv4 and IPv6 egress addresses.
pub fn local_egress_addresses() -> Vec<IpAddr> {
    [probe_egress(V4_PROBE), probe_egress(V6_PROBE)]
        .into_iter()
        .flatten()
        .filter(is_globally_routable)
        .collect()
}

/// UDP-connects to a well-known anchor and reads back the source address the
/// kernel chose. No packet leaves the host; the route lookup is real.
fn probe_egress(probe: SocketAddr) -> Option<IpAddr> {
    let bind: SocketAddr = match probe {
        SocketAddr::V4(_) => "0.0.0.0:0".parse().ok()?,
        SocketAddr::V6(_) => "[::]:0".parse().ok()?,
    };
    let sock = UdpSocket::bind(bind).ok()?;
    sock.connect(probe).ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}
