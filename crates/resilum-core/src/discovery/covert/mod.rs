//! Covert-carrier discovery: announce presence, rendezvous for the endpoint,
//! attach a per-peer covert interface in-process at runtime.

mod addresses;
pub mod endpoint;
mod inproc;
mod plugin;
pub mod rendezvous;

pub use addresses::AddressSource;
pub use plugin::CovertDiscovered;
