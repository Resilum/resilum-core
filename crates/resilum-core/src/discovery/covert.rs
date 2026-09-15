//! Covert-carrier discovery: announce presence, rendezvous for the endpoint,
//! attach a per-peer covert interface in-process at runtime.

pub use self::addresses::{AddressSource, DialableAddress, Reach};
#[cfg(target_os = "linux")]
pub use self::inproc::listen;
pub use self::plugin::CovertDiscovered;

mod addresses;
pub mod endpoint;
mod inproc;
mod plugin;
pub mod rendezvous;
