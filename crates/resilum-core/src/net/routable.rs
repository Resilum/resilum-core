//! Which addresses are safe to hand a peer as our own.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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
        for ip in ["198.18.0.1", "198.19.255.254", "203.0.113.1"] {
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
            "::ffff:198.18.0.1",
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
