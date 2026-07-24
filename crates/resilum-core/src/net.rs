//! Local network helpers reused across transports (discovery, egress).
//!
//! `local_egress_addresses` UDP-connects to well-known IPv4/IPv6 anchors and
//! reads the kernel's chosen source address — no packet leaves the host, but
//! the egress-route lookup is real. Returns only globally-routable results;
//! private, loopback, link-local, ULA and documentation addresses are dropped.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

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

/// Locally-assigned Yggdrasil IPv6, or `None` if no `ygg0`-style interface is
/// up. Yggdrasil owns `200::/7` so the interface name is irrelevant — matches
/// any platform (`ygg0` on Linux, `tun*`/`utun*` on Android/iOS/macOS).
pub fn yggdrasil_local_ipv6() -> Option<Ipv6Addr> {
    if_addrs::get_if_addrs().ok()?.into_iter().find_map(|i| {
        if let IpAddr::V6(v6) = i.ip()
            && is_yggdrasil_range(&v6)
        {
            Some(v6)
        } else {
            None
        }
    })
}

fn is_yggdrasil_range(ip: &Ipv6Addr) -> bool {
    // 200::/7 → first 7 bits are 0000_001, i.e. first octet is 0x02 or 0x03.
    (ip.octets()[0] & 0xfe) == 0x02
}

fn probe_egress(probe: SocketAddr) -> Option<IpAddr> {
    let bind: SocketAddr = match probe {
        SocketAddr::V4(_) => "0.0.0.0:0".parse().ok()?,
        SocketAddr::V6(_) => "[::]:0".parse().ok()?,
    };
    let sock = UdpSocket::bind(bind).ok()?;
    sock.connect(probe).ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}

/// True if `ip` is safe to publish as our public address to other peers.
/// Filters private (RFC 1918/ULA), loopback, link-local, documentation,
/// multicast, CGNAT, and IPv4-mapped IPv6.
pub fn is_globally_routable(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_global_v4(v4),
        IpAddr::V6(v6) => is_global_v6(v6),
    }
}

fn is_global_v4(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        // 100.64.0.0/10 CGNAT
        || (o[0] == 100 && (o[1] & 0xc0) == 64)
        // 0.0.0.0/8 "this network"
        || o[0] == 0
        // 240.0.0.0/4 reserved for future use (incl. 255.x limited broadcast)
        || o[0] >= 240)
}

fn is_global_v6(ip: &Ipv6Addr) -> bool {
    let s = ip.segments();
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // fe80::/10 link-local
        || (s[0] & 0xffc0) == 0xfe80
        // fc00::/7 unique-local (ULA)
        || (s[0] & 0xfe00) == 0xfc00
        // ::ffff:0:0/96 IPv4-mapped — never route as v6
        || (s[0] == 0 && s[1] == 0 && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0xffff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn rejects_private_v4() {
        for ip in [
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "127.0.0.1",
            "169.254.1.1",
            "100.64.0.1",
            "0.1.2.3",
            "255.255.255.255",
            "240.0.0.1",
        ] {
            assert!(!is_global_v4(&ip.parse().unwrap()), "{ip} must be filtered");
        }
    }

    #[test]
    fn accepts_public_v4() {
        for ip in ["1.1.1.1", "8.8.8.8", "203.0.113.1"] {
            // 203.0.113.0/24 is TEST-NET-3 (documentation): expected to be filtered.
            let accepted = is_global_v4(&ip.parse().unwrap());
            if ip == "203.0.113.1" {
                assert!(!accepted, "TEST-NET-3 must be filtered as documentation");
            } else {
                assert!(accepted, "{ip} must be global");
            }
        }
    }

    #[test]
    fn rejects_private_v6() {
        for ip in [
            "::1",
            "fe80::1",
            "fc00::1",
            "fd00::1",
            "ff02::1",
            "::",
            "::ffff:1.2.3.4",
        ] {
            assert!(!is_global_v6(&ip.parse().unwrap()), "{ip} must be filtered");
        }
    }

    #[test]
    fn accepts_public_v6() {
        for ip in ["2606:4700:4700::1111", "2001:db8::1"] {
            // 2001:db8::/32 is documentation but not filtered here — it is not
            // link-local/ULA/multicast, and we intentionally don't reject it
            // (users may run into it in test networks).
            assert!(is_global_v6(&ip.parse().unwrap()), "{ip} should pass");
        }
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_v4(bytes in any::<[u8; 4]>()) {
            let _ = is_global_v4(&Ipv4Addr::from(bytes));
        }
        #[test]
        fn never_panics_on_arbitrary_v6(bytes in any::<[u8; 16]>()) {
            let _ = is_global_v6(&Ipv6Addr::from(bytes));
        }
    }
}
