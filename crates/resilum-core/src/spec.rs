//! Config specs (bridge / VPN / covert) and their YAML loader.

mod bridge;
mod common;
mod covert;
mod parse;
mod vpn;

pub use self::bridge::{BridgeMode, BridgeSpec};
pub use self::covert::CovertSpec;
pub use self::parse::{Specs, load, siblings_for};
pub use self::vpn::{VpnMode, VpnSpec};
