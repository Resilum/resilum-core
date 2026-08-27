//! Which addresses are safe to hand a peer as our own, and which are safe to
//! dial when a peer names one.

#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// True if `ip` is safe to publish as our public address to other peers.
/// Filters private (RFC 1918/ULA), loopback, link-local, documentation,
/// multicast, CGNAT, and IPv4-mapped IPv6.
pub fn is_globally_routable(ip: &IpAddr) -> bool {
    names_one_host(ip) && !is_local_scope(ip)
}

pub fn names_one_host(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => names_one_host_v4(v4),
        IpAddr::V6(v6) => names_one_host_v6(v6),
    }
}

fn is_local_scope(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_local_scope_v4(v4),
        IpAddr::V6(v6) => is_local_scope_v6(v6),
    }
}

fn names_one_host_v4(ip: &Ipv4Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || is_this_network(ip)
        || is_reserved_for_future_use(ip))
}

fn is_local_scope_v4(ip: &Ipv4Addr) -> bool {
    ip.is_private() || ip.is_link_local() || ip.is_documentation() || is_carrier_grade_nat(ip)
}

fn names_one_host_v6(ip: &Ipv6Addr) -> bool {
    !(ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || ip.to_ipv4_mapped().is_some())
}

fn is_local_scope_v6(ip: &Ipv6Addr) -> bool {
    is_link_local_v6(ip) || is_unique_local_v6(ip)
}

fn is_this_network(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 0
}

fn is_reserved_for_future_use(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] >= 240
}

fn is_carrier_grade_nat(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (o[1] & 0xc0) == 64
}

fn is_link_local_v6(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn is_unique_local_v6(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}
