//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed directly by a Rust caller, or over the C ABI through `resilum-ffi`.

pub mod announce_cap;
pub mod announce_ours;
pub mod announce_payload;
pub mod announce_trigger;
pub mod ble;
mod bridge;
mod config;
mod config_file;
pub mod coordinates;
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
pub mod letting_go;
pub mod link;
pub mod lxmf;
pub mod mirrors;
pub mod net;
mod node;
pub mod pump;
mod socks5_tcp;
pub mod spec;
pub mod status;
pub mod text;
#[cfg(feature = "arti")]
pub mod tor;
mod wall_clock;
#[cfg(unix)]
pub mod wifi_group;
#[cfg(feature = "ygg")]
pub mod ygg;

pub use leviculum_std::socket_hook::OutboundSocketHook;

pub use self::config::{
    BleInterface, Config, CovertDiscoveryService, DiscoveryService, EgressListen, EndpointFormat,
    I2pInterface, IngressConfig, LxmfConfig, UdpInterface, WifiGroup,
};
pub use self::config_file::{from_json, from_yaml};
#[cfg(unix)]
pub use self::egress::vpn::VpnHandle;
pub use self::error::{Error, Result};
pub use self::event::Event;
#[cfg(feature = "iroh")]
pub use self::iroh::IrohHandle;
pub use self::node::Node;
#[cfg(feature = "ygg")]
pub use self::ygg::YggHandle;
