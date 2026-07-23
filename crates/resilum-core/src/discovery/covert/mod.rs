//! Covert-carrier discovery: announce presence, rendezvous for the endpoint,
//! attach a per-peer covert PipeInterface at runtime.

pub mod endpoint;
mod plugin;
pub mod rendezvous;

pub use plugin::CovertDiscovered;
