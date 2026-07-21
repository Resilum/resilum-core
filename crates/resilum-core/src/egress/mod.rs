//! Egress candidate model, eligibility filtering and selection policy.

pub mod active;
mod candidate;
pub mod connect;
pub mod discover;
mod eligibility;
pub mod listen;
pub mod monitor;
pub mod probe;
mod selector;

pub use active::ActiveLinks;
pub use candidate::{Candidate, CandidateRegistry};
pub use eligibility::eligible;
pub use selector::choose_best;
