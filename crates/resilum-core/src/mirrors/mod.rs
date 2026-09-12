//! Mesh-side auto-discovery of rngit mirror nodes: this node announces the
//! repos it hosts as rngit mirrors together with the rngit Repositories
//! Destination hash to dial, peers do the same, everyone keeps a local
//! registry so downloaders can pick a live mirror without out-of-band URL
//! sharing.

mod consume;
mod handover;
mod payload;
mod produce;
mod registry;
mod store;

pub use consume::run_consume;
pub use handover::SharedWithRngit;
pub use payload::name_hash;
pub use produce::run_produce;
pub use registry::{Entry, Registry};

pub(crate) const APP_NAME: &str = "resilum";
pub(crate) const ASPECT: &[&str] = &["mirrors", "list"];

#[cfg(test)]
mod tests;
