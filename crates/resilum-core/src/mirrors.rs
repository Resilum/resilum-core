//! Mesh-side auto-discovery of rngit mirror nodes: this node announces the
//! repos it hosts as rngit mirrors together with the rngit Repositories
//! Destination hash to dial, peers do the same, everyone keeps a local
//! registry so downloaders can pick a live mirror without out-of-band URL
//! sharing.

pub use self::consume::run_consume;
pub use self::handover::SharedWithRngit;
pub use self::payload::name_hash;
pub use self::produce::run_produce;
pub use self::registry::{Entry, Registry};

mod consume;
mod handover;
mod payload;
mod produce;
mod registry;

pub(crate) const APP_NAME: &str = "resilum";
pub(crate) const ASPECT: &[&str] = &["mirrors", "list"];

#[cfg(test)]
mod tests;
