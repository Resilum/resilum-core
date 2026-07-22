//! Covert transport: bidirectional bytes tunnelled inside carrier packets via
//! a carrier-agnostic session engine on top. Extensible by carrier — an
//! implementation is a `Carrier` impl plus its wire encoding.
//!
//! Runtime prerequisites by carrier (implementation-time, not policy):
//! - `icmp` — client works everywhere via `SOCK_DGRAM/IPPROTO_ICMP` (Linux,
//!   Android with `INTERNET` permission, iOS without entitlements); server
//!   side needs raw sockets (root or `CAP_NET_RAW`).
//! - `dns` — plain UDP :53 as a client, works from any non-privileged app.
//! - `arp`, `ntp`, `dhcp`, `snmp` — planned; either require raw sockets or
//!   privileged ports, or on mobile ride the packet-tunnel API
//!   (`VpnService` / `NEPacketTunnelProvider`).

pub mod carrier;
pub mod engine;
pub mod icmp;
pub mod runner;
