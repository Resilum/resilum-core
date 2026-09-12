//! ICMP-echo covert carrier. The server side needs raw sockets, AF_PACKET, a
//! BPF filter, and netfilter suppression of the kernel's own echo-reply.

pub mod client;
pub mod marker;
#[cfg(target_os = "linux")]
pub mod nftguard;
#[cfg(target_os = "linux")]
pub mod server;
pub mod socket;
#[cfg(unix)]
pub mod wake;
pub mod wire;

pub use client::IcmpClient;
#[cfg(target_os = "linux")]
pub use server::IcmpServer;
