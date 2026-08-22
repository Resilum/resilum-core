//! Egress candidate model, eligibility filtering and selection policy.

pub mod active;
mod candidate;
pub mod discover;
mod eligibility;
pub mod ingress;
pub mod listen;
pub mod monitor;
pub mod own;
pub mod probe;
mod relay;
mod selector;
mod socks5;
pub mod vpn;

pub use active::ActiveLinks;
pub use candidate::{Candidate, CandidateRegistry};
pub use eligibility::{allowed, eligible};
pub use selector::choose_best;
