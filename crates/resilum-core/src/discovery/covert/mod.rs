//! Covert-carrier discovery: announce presence, rendezvous for the endpoint,
//! attach a per-peer covert PipeInterface at runtime.

mod addresses;
mod attach;
pub mod endpoint;
mod plugin;
pub mod rendezvous;

pub use addresses::AddressSource;
pub use plugin::CovertDiscovered;
