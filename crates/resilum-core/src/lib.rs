//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed by `resilumd` and, via `resilum-ffi`, the mobile app.

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
pub mod identity;
pub mod link;
pub mod mirrors;
pub mod net;
mod node;
pub mod pump;
pub mod spec;
pub mod supervisor;
#[cfg(feature = "arti")]
pub mod tor;

pub use config::{
    Config, DiscoveryService, EgressListen, EndpointFormat, I2pInterface, IngressConfig,
};
pub use config_file::{from_json, from_yaml};
pub use error::{Error, Result};
pub use event::Event;
pub use node::Node;

#[cfg(unix)]
pub use egress::vpn::VpnHandle;
