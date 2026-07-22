//! ICMP-echo covert carrier.
//!
//! Client is portable (`SOCK_DGRAM+IPPROTO_ICMP`, no root, mobile-friendly).
//! Server side (raw sockets, AF_PACKET, BPF filter, netfilter suppression of
//! the kernel's own echo-reply) is Linux-only and lands in a follow-up.

pub mod client;
pub mod id;
pub mod wire;

pub use client::IcmpClient;
pub use id::tunnel_id;
