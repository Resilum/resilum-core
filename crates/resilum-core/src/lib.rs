//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed directly by a Rust caller, or over the C ABI through `resilum-ffi`.

pub mod announce_cap;
pub mod announce_payload;
pub mod announce_trigger;
mod bridge;
mod config;
mod config_file;
pub mod covert;
mod defaults;
pub mod discovery;
pub mod dispatch;
pub mod egress;
mod engine;
mod error;
mod event;
pub mod hex;
pub mod identity;
#[cfg(feature = "iroh")]
pub mod iroh;
pub mod link;
pub mod lxmf;
pub mod mirrors;
pub mod net;
mod node;
pub mod pump;
#[cfg(any(feature = "arti", feature = "ygg"))]
mod socks5_tcp;
pub mod spec;
pub mod supervisor;
#[cfg(feature = "arti")]
pub mod tor;
mod wall_clock;
#[cfg(feature = "ygg")]
pub mod ygg;

pub use config::{
    Config, DiscoveryService, EgressListen, EndpointFormat, I2pInterface, IngressConfig, LxmfConfig,
};
pub use config_file::{from_json, from_yaml};
pub use error::{Error, Result};
pub use event::Event;
#[cfg(feature = "iroh")]
pub use iroh::IrohHandle;
pub use node::Node;
#[cfg(feature = "ygg")]
pub use ygg::YggHandle;

pub use leviculum_std::socket_hook::OutboundSocketHook;

#[cfg(unix)]
pub use egress::vpn::VpnHandle;
