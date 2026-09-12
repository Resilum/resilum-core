//! Config specs (bridge / VPN / covert) and their YAML loader.

mod bridge;
mod common;
mod covert;
mod parse;
mod vpn;

pub use bridge::{BridgeMode, BridgeSpec};
pub use covert::CovertSpec;
pub use parse::{Specs, load, siblings_for};
pub use vpn::{VpnMode, VpnSpec};
