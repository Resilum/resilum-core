//! Covert transport: bidirectional bytes tunnelled inside carrier packets via
//! a carrier-agnostic session engine on top. Extensible by carrier — an
//! implementation is a `Carrier` impl plus its wire encoding.
//!
//! Runtime prerequisites by carrier (implementation-time, not policy):
//! - `icmp` — client works on Linux + Android non-root via
//!   `SOCK_DGRAM/IPPROTO_ICMP`; server needs raw sockets (root or `CAP_NET_RAW`).
//! - `dns` — plain UDP :53, works from a non-privileged app (like iodine).
//! - `arp`, `ntp`, `dhcp`, `snmp` — planned; either require raw / privileged
//!   ports, or ride the packet-tunnel API (`VpnService` / `NEPacketTunnelProvider`).

pub mod carrier;
pub mod icmp;
