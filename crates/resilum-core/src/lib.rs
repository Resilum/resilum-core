//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed by `resilumd` and, via `resilum-ffi`, the mobile app.

pub mod announce_cap;
pub mod announce_payload;
pub mod announce_trigger;
mod bridge;
mod config;
mod config_file;
mod defaults;
pub mod discovery;
pub mod dispatch;
pub mod egress;
mod engine;
mod error;
mod event;
mod identity;
pub mod link;
mod node;
pub mod pump;
pub mod spec;
pub mod supervisor;

pub use config::{
    Config, ConnectConfig, DiscoveryService, EgressListen, EndpointFormat, I2pInterface,
};
pub use config_file::from_yaml;
pub use error::{Error, Result};
pub use event::Event;
pub use node::Node;
